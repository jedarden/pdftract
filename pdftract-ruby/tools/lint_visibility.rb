# frozen_string_literal: true

#
# Static lint check for Ruby visibility-keyword bugs in generated SDK output.
#
# The bug class this catches (pdftract-ruby audit, 2026-07-20, bug #2): the
# codegen template emits a bare `private` above the internal helpers and never
# restores `public` before the contract-method block, so every contract method
# in Client is unintentionally private and every README example raises
# NoMethodError. The structure reads perfectly in code review — only invoking
# the code (or this check) reveals it. Root cause lives in the pdftract repo's
# Tera template (templates/sdk-skeleton/ruby/lib/pdftract/codegen/methods.rb.tera);
# this lint is the safety net that fails in seconds when a codegen run
# regenerates the bug, instead of waiting for a containerized conformance run.
#
# How it decides: a Ruby visibility keyword applies to every method defined
# after it in the same class body, until another keyword changes it. So instead
# of counting keywords, this walks the parsed file (Ripper, stdlib — no gems,
# runs in the bare ruby:3.2-slim CI image) and records the effective visibility
# of each defined method. Any method that ends up non-public and is not
# explicitly allowlisted is a violation. Default-deny is deliberate: generated
# output has no hand-written exceptions, so a *new* contract method emitted by
# a template change is caught automatically rather than silently passing a
# stale list.
#
# This is a pre-filter, not a gate. It cannot catch the require_relative path
# bug or a stubbed method body — only runtime conformance execution
# (pdftract-ruby/docs/plan/plan.md ADR-001) can. Run both.
#
# Usage:
#   ruby tools/lint_visibility.rb [options] PATH [PATH...]
#
# Options:
#   --allow-private NAME    Permit NAME to be non-public (repeatable)
#   --allow-private-file F  Read additional permitted names from F (one per line)
#   --no-default-allowlist  Drop the built-in internal-helper allowlist
#   -h, --help              Show this help
#
# PATH may be a .rb file or a directory (scanned recursively for *.rb, skipping
# test/spec/vendor trees — the target is generated lib/ output).
#
# Exit codes:
#   0  no violations
#   1  violations found (including files Ripper could not parse)
#   2  usage or IO error
#
# Bead: bf-1uhnlv
# Plan: pdftract-ruby/docs/plan/plan.md ADR-001, Alternative 3

require "ripper"

module PdftractVisibilityLint
  # Helpers the generated template intentionally keeps private (executes the
  # binary, maps exit codes to error classes). Everything else in a generated
  # file must be public.
  DEFAULT_ALLOWLIST = %w[exec map_error].freeze

  # `initialize` is always effectively private in Ruby; never report it.
  NEVER_REPORTED = %w[initialize].freeze

  VISIBILITY_KEYWORDS = {
    "private" => :private,
    "public" => :public,
    "protected" => :protected,
  }.freeze

  CLASS_METHOD_KEYWORDS = {
    "private_class_method" => :private,
    "public_class_method" => :public,
  }.freeze

  ACCESSOR_METHODS = %w[attr_reader attr_writer attr_accessor].freeze

  # Trees that are not generated lib/ output.
  SKIP_PATH = %r{(^|/)(test|spec|vendor)(/|$)}

  Violation = Struct.new(:file, :line, :name, :visibility, :switch_line, keyword_init: true)

  # Visibility state for one class/module body. A bare `private` inside a
  # class body affects that class only, so state is pushed/popped per scope.
  class Scope
    attr_reader :instance_visibility, :class_visibility
    attr_reader :instance_switch_line, :class_switch_line

    def initialize
      # Default visibility at the top of any body is public.
      @instance_visibility = :public
      @class_visibility = :public
      @instance_switch_line = nil
      @class_switch_line = nil
    end

    def switch(kind, visibility, line)
      if kind == :class_method
        @class_visibility = visibility
        @class_switch_line = line
      else
        @instance_visibility = visibility
        @instance_switch_line = line
      end
    end

    def visibility_for(singleton)
      singleton ? @class_visibility : @instance_visibility
    end

    def switch_line_for(singleton)
      singleton ? @class_switch_line : @instance_switch_line
    end
  end

  class << self
    # Effective visibility of every method defined in +source+.
    # Returns a list of hashes: { name:, line:, visibility:, switch_line:, singleton: }.
    # Returns nil if the source cannot be parsed (Ripper returned no sexp).
    def analyze(source)
      sexp = Ripper.sexp(source)
      return nil if sexp.nil?

      methods = []
      walk(sexp, Scope.new, methods)
      methods
    end

    # Lint +paths+ (files or directories). Returns a hash:
    #   { violations:, parse_errors:, files_scanned: }
    def lint_path(paths, allowlist: DEFAULT_ALLOWLIST)
      violations = []
      parse_errors = []
      files_scanned = []

      each_ruby_file(paths) do |file|
        methods = analyze(File.read(file))
        files_scanned << file

        if methods.nil?
          parse_errors << file
          next
        end

        methods.each do |m|
          next if m[:visibility] == :public
          next if allowed?(m[:name], allowlist)

          violations << Violation.new(
            file: file,
            line: m[:line],
            name: m[:name],
            visibility: m[:visibility],
            switch_line: m[:switch_line],
          )
        end
      end

      { violations: violations, parse_errors: parse_errors, files_scanned: files_scanned }
    end

    private

    def allowed?(name, allowlist)
      NEVER_REPORTED.include?(name) || allowlist.include?(name)
    end

    def each_ruby_file(paths)
      files = paths.flat_map do |path|
        if File.directory?(path)
          Dir.glob(File.join(path, "**", "*.rb")).sort
        elsif File.file?(path)
          [path]
        else
          raise Errno::ENOENT, path
        end
      end

      files.uniq.each do |file|
        next if SKIP_PATH.match?(file)

        yield file
      end
    end

    # --- sexp walking -------------------------------------------------------

    def walk(node, scope, out)
      return unless node.is_a?(Array)

      case node[0]
      when :class
        # [:class, path, superclass, body] — superclass is node[2]
        walk(node[3], Scope.new, out)
      when :module
        # [:module, path, body]
        walk(node[2], Scope.new, out)
      when :sclass
        # [:sclass, target, body]
        walk(node[2], Scope.new, out)
      when :def
        # [:def, name_sexp, params, body]
        record(out, ident_name(node[1]), line_of(node[1]), scope, false)
        walk(node[3], scope, out)
      when :defs
        # [:defs, target, op, name_sexp, params, body]
        record(out, ident_name(node[3]), line_of(node[3]), scope, true)
        walk(node[5], scope, out)
      when :vcall
        # [:vcall, ident_sexp] — a bare word statement: `private`
        handle_visibility_word(node[1], scope)
      when :command
        walk_command(node, scope, out)
      when :method_add_arg
        # [:method_add_arg, fcall, arg_paren] — the parenthesized call form,
        # e.g. attr_accessor(:c) or private(). Not a :command.
        name = ident_name(node[1].is_a?(Array) ? node[1][1] : nil)
        if VISIBILITY_KEYWORDS.key?(name) || CLASS_METHOD_KEYWORDS.key?(name)
          apply_visibility_call(name, node[2], scope, out, line_of(node[1][1]))
        elsif ACCESSOR_METHODS.include?(name) && node[2]
          symbol_nodes(node[2]).each { |sym| record(out, sym[:name], sym[:line], scope, false) }
        else
          node.each { |child| walk(child, scope, out) }
        end
      else
        node.each { |child| walk(child, scope, out) }
      end
    end

    # `ident args` with no receiver: `private :a`, `attr_reader :x`,
    # `private def foo`, or any ordinary call.
    def walk_command(node, scope, out)
      ident = node[1]
      name = ident_name(ident)

      if VISIBILITY_KEYWORDS.key?(name) || CLASS_METHOD_KEYWORDS.key?(name)
        apply_visibility_call(name, node[2], scope, out, line_of(ident))
      elsif ACCESSOR_METHODS.include?(name)
        symbol_nodes(node[2]).each { |sym| record(out, sym[:name], sym[:line], scope, false) }
      else
        # Ordinary call — keep descending so a def inside a block is still seen.
        walk(node[2], scope, out)
      end
    end

    def apply_visibility_call(name, args, scope, out, keyword_line)
      kind = CLASS_METHOD_KEYWORDS.key?(name) ? :class_method : :instance
      visibility = VISIBILITY_KEYWORDS.fetch(name) { CLASS_METHOD_KEYWORDS[name] }
      arg_nodes = unwrap_args(args)

      if arg_nodes.empty?
        # Bare keyword: switches visibility for the rest of this body.
        scope.switch(kind, visibility, keyword_line)
      else
        arg_nodes.each do |arg|
          if kind == :class_method
            walk_class_method_argument(arg, scope, out, keyword_line)
          else
            walk_visibility_argument(arg, scope, out, keyword_line)
          end
        end
      end
    end

    # Positional argument nodes of a call's argument sexp, unwrapping the
    # parenthesized form so `private()` is recognized as the bare keyword.
    def unwrap_args(args)
      nodes = args.is_a?(Array) ? args[1..].compact : []
      return [] if nodes.empty?

      first = nodes[0]
      if nodes.size == 1 && first.is_a?(Array) && first[0] == :arg_paren
        inner = first[1]
        if inner.is_a?(Array) && inner[0] == :args_add_block && inner[1].is_a?(Array)
          return inner[1].compact
        end
        return []
      end
      nodes
    end

    # Argument to a visibility keyword call:
    #   `private def foo` — that one method, section state unchanged
    #   `private :foo` — explicitly sanctioned private, section state unchanged
    def walk_visibility_argument(arg, scope, out, keyword_line)
      return unless arg.is_a?(Array)

      case arg[0]
      when :def
        record(out, ident_name(arg[1]), position_of(arg[1]), scope, false) do
          Scope.new.tap { |s| s.switch(:instance, :private, keyword_line) }
        end
        walk(arg[3], scope, out)
      when :defs
        record(out, ident_name(arg[3]), position_of(arg[3]), scope, true) do
          Scope.new.tap { |s| s.switch(:class_method, :private, keyword_line) }
        end
        walk(arg[5], scope, out)
      when :symbol_literal
        # Named method — an explicit, intentional visibility choice, not a leak.
      else
        arg.each { |child| walk_visibility_argument(child, scope, out, keyword_line) }
      end
    end

    def walk_class_method_argument(arg, scope, out, keyword_line)
      return unless arg.is_a?(Array)

      case arg[0]
      when :defs
        record(out, ident_name(arg[3]), position_of(arg[3]), scope, true) do
          Scope.new.tap { |s| s.switch(:class_method, :private, keyword_line) }
        end
        walk(arg[5], scope, out)
      when :symbol_literal
        # Named singleton method — explicit choice, not a leak.
      else
        arg.each { |child| walk_class_method_argument(child, scope, out, keyword_line) }
      end
    end

    def handle_visibility_word(ident, scope)
      name = ident_name(ident)
      if VISIBILITY_KEYWORDS.key?(name)
        scope.switch(:instance, VISIBILITY_KEYWORDS[name], line_of(ident))
      elsif CLASS_METHOD_KEYWORDS.key?(name)
        scope.switch(:class_method, CLASS_METHOD_KEYWORDS[name], line_of(ident))
      end
    end

    # Records a method with the scope's current visibility. +scope_override+
    # supplies the visibility for inline forms (`private def foo`) whose
    # section state must not leak to the following methods.
    def record(out, name, line, scope, singleton)
      return if name.nil? || line.nil?

      if block_given?
        scope = yield
      end

      out << {
        name: name,
        line: line,
        visibility: scope.visibility_for(singleton),
        switch_line: scope.switch_line_for(singleton),
        singleton: singleton,
      }
    end

    # All `:symbol` name/line pairs anywhere inside an argument list.
    def symbol_nodes(node)
      found = []
      return found unless node.is_a?(Array)

      if node[0] == :symbol
        name = ident_name(node[1])
        found << { name: name, line: line_of(node[1]) } if name
      else
        node.each { |child| found.concat(symbol_nodes(child)) }
      end
      found
    end

    def ident_name(sexp)
      return nil unless sexp.is_a?(Array)
      return sexp[1].to_s if %i[@ident @op @const].include?(sexp[0])

      nil
    end

    # Ripper positions are [lineno, column].
    def position_of(sexp)
      return nil unless sexp.is_a?(Array)
      return sexp[2] if %i[@ident @op @const].include?(sexp[0])

      nil
    end

    def line_of(sexp)
      pos = position_of(sexp)
      pos && pos[0]
    end
  end
end

# --- CLI ----------------------------------------------------------------------

if $PROGRAM_NAME == __FILE__
  require "optparse"

  allowlist = PdftractVisibilityLint::DEFAULT_ALLOWLIST.dup

  parser = OptionParser.new do |opts|
    opts.banner = "Usage: ruby #{File.basename($PROGRAM_NAME)} [options] PATH [PATH...]"
    opts.separator "Static lint for Ruby visibility-keyword bugs in generated SDK output."
    opts.separator "Any method that ends up non-public and is not allowlisted is a violation."
    opts.separator ""
    opts.on("--allow-private NAME", "Permit NAME to be non-public (repeatable)") { |v| allowlist << v }
    opts.on("--allow-private-file FILE", "Read permitted names from FILE (one per line)") do |v|
      File.readlines(v).each do |line|
        name = line.strip
        allowlist << name unless name.empty? || name.start_with?("#")
      end
    end
    opts.on("--no-default-allowlist", "Drop the built-in internal-helper allowlist") { allowlist = [] }
    opts.on("-h", "--help", "Show this help") { puts opts; exit 0 }
  end
  parser.parse!

  paths = ARGV
  if paths.empty?
    warn parser.to_s
    exit 2
  end

  begin
    result = PdftractVisibilityLint.lint_path(paths, allowlist: allowlist)
  rescue Errno::ENOENT => e
    warn "error: #{e.message}"
    exit 2
  end

  result[:parse_errors].each do |file|
    puts "#{file}: ERROR: could not parse Ruby source — generated output may be syntactically invalid"
  end

  result[:violations].each do |v|
    where = v.switch_line ? " (set by visibility keyword at line #{v.switch_line})" : ""
    puts "#{v.file}:#{v.line}: method `#{v.name}' is #{v.visibility}#{where}"
    puts "  generated SDK methods must be public; add `public' before the contract-method block,"
    puts "  or allowlist the name with --allow-private if it is intentionally internal"
  end

  if result[:violations].empty? && result[:parse_errors].empty?
    puts "OK: #{result[:files_scanned].size} file(s) scanned, no visibility-keyword violations"
    exit 0
  else
    total = result[:violations].size + result[:parse_errors].size
    bad_files = (result[:violations].map(&:file) | result[:parse_errors]).size
    puts "#{total} violation(s) in #{bad_files} file(s) of #{result[:files_scanned].size} scanned"
    exit 1
  end
end
