# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name          = "pdftract"
  spec.version       = "1.0.0"
  spec.authors       = ["jedarden"]
  spec.email         = ["jedarden@example.com"]

  spec.summary       = "PDFtract SDK - PDF extraction and conformance testing for Ruby"
  spec.description   = "Ruby SDK for pdftract - PDF extraction, OCR, and conformance testing"
  spec.homepage      = "https://github.com/jedarden/pdftract"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.2.0"

  spec.files = Dir["{lib}/**/*", "LICENSE", "README.md", "GENERATED"]
  spec.require_paths = ["lib"]

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
