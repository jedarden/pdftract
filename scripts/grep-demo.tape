# Regenerate with: scripts/record-grep-demo.sh
# The command below is the exact fixture-backed grep invocation verified by
# pdftract-963feba8. It intentionally uses JSON so bbox and confidence are
# visible in the recording.

Output grep-demo.gif

Require pdftract

Set Shell "bash"
Set FontFamily "DejaVu Sans Mono"
Set FontSize 20
Set Width 1440
Set Height 620
Set Padding 24
Set MarginFill "#0d1117"
Set Margin 0
Set WindowBar Colorful
Set WindowBarSize 32
Set Framerate 30
Set TypingSpeed 8ms

Type "pdftract grep --json --no-progress -E 'Lorem (ipsum|dolor)' tests/fixtures/grep-corpus/corpus/synthetic_1.pdf" Sleep 500ms Enter
Sleep 3s
