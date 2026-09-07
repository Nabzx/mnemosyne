# Mnemosyne README demo. Regenerate from the repo root with `just demo`
# (builds the mnem binary, then renders this tape with vhs).

Output assets/demo.gif

Set Shell bash
Set FontFamily "JetBrains Mono"
Set FontSize 16
Set Width 1280
Set Height 544
Set Padding 44
Set WindowBar Colorful
Set Theme "Catppuccin Mocha"
Set TypingSpeed 24ms
Set Framerate 24

Hide
Type `export PATH="$PWD/target/release:$PATH"`
Enter
Type `PROMPT_COMMAND=''; export PS1="\[\e[1;38;2;203;166;247m\]agent\[\e[0m\] \[\e[38;2;137;180;250m\]▸\[\e[0m\] "`
Enter
Type `cd "$(mktemp -d)" && mnem init >/dev/null && clear`
Enter
Sleep 300ms
Show
Sleep 600ms

Type "# an agent works a billing case, recording what it learns"
Enter
Type `mnem add customer-4821 "on the Enterprise plan" --source ticket-4821`
Enter
Sleep 600ms
Type `mnem commit -m "open the case" --author agent`
Enter
Sleep 1400ms

Type "# fork the memory to test a hunch — the main line keeps its own"
Enter
Type "mnem checkout -b what-if-downgraded"
Enter
Sleep 500ms
Type `mnem add customer-4821 "downgraded to Pro"`
Enter
Sleep 400ms
Type "mnem diff"
Enter
Sleep 1900ms
Type `mnem commit -m "explore: what if they downgraded" --author agent`
Enter
Sleep 700ms
Type "mnem checkout main"
Enter
Sleep 1500ms

Type "# later, the agent misreads a billing note and commits it for real"
Enter
Type `mnem add customer-4821 "downgraded to Pro last month" --source billing-note-8842 --step step-31`
Enter
Sleep 500ms
Type `mnem commit -m "reconcile the plan tier from billing" --author agent`
Enter
Sleep 600ms
Type `mnem add memo "flag the billing mismatch" && mnem commit -m "keep working" --author agent`
Enter
Sleep 1400ms

Type "# an hour on, a wrong answer. find the commit where it went wrong:"
Enter
Type `mnem bisect --node customer-4821 --equals '"downgraded to Pro last month"'`
Enter
Sleep 3200ms

Type "# and trace that belief to the observation behind it:"
Enter
Type "mnem blame customer-4821"
Enter
Sleep 4000ms
