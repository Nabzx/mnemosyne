# Demo recording for the README.
# Regenerate from the repo root:
#   cargo build --release -p mnem-cli
#   vhs assets/demo.tape
# (or `just demo`)

Output assets/demo.gif

Set Shell bash
Set FontSize 15
Set Width 1120
Set Height 760
Set Padding 24
Set Theme "Dracula"
Set TypingSpeed 32ms

Hide
Type `export PATH="$PWD/target/release:$PATH"; export PS1="$ "; cd "$(mktemp -d)"; clear`
Enter
Sleep 400ms
Show

Type "mnem init"
Enter
Sleep 800ms

Type "# the agent records what it learns, with where it came from"
Enter
Type `mnem add customer-4821 "on the Enterprise plan" --source ticket-4821`
Enter
Sleep 500ms
Type `mnem add case-4821 "billing dispute, investigating"`
Enter
Sleep 500ms
Type `mnem commit -m "open the case" --author agent`
Enter
Sleep 1000ms

Type "# fork the memory to test an assumption, without touching the main line"
Enter
Type `mnem checkout -b assume-downgrade`
Enter
Sleep 700ms
Type `mnem add customer-4821 "downgraded to Pro last month"`
Enter
Sleep 500ms

Type "# what would change under that assumption"
Enter
Type "mnem diff"
Enter
Sleep 1800ms
Type `mnem commit -m "assume the downgrade" --author agent`
Enter
Sleep 900ms

Type "# back to the main line: untouched"
Enter
Type "mnem checkout main"
Enter
Sleep 500ms
Type "mnem show"
Enter
Sleep 1500ms

Type "# compare the two lines of memory"
Enter
Type "mnem diff main assume-downgrade"
Enter
Sleep 3500ms
