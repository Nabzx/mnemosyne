# Mnemosyne README demo. Regenerate from the repo root with `just demo`
# (builds the Python SDK, then renders this tape with vhs).
#
# The terminal runs examples/claude_agent.py in replay mode: a support agent
# diagnoses a wrong answer with bisect and blame, then corrects the record.
# The store, the commit ids, the bisect and the blame are real; the agent's
# lines are a fixed script. Run the example with --live and an API key for a
# real Claude tool-use loop.

Output assets/demo.gif

Set Shell bash
Set FontFamily "JetBrains Mono"
Set FontSize 16
Set Width 1400
Set Height 720
Set Padding 44
Set WindowBar Colorful
Set Theme "Catppuccin Mocha"
Set TypingSpeed 22ms
Set Framerate 24

Hide
Type `export VIRTUAL_ENV_DISABLE_PROMPT=1`
Enter
Type `source .venv/bin/activate 2>/dev/null`
Enter
Type `PROMPT_COMMAND=''; export PS1=""; clear`
Enter
Sleep 300ms
Show
Sleep 500ms

Type `python examples/claude_agent.py --demo`
Sleep 400ms
Enter
Sleep 21s
