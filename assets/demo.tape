# Demo recording for the README.
# Regenerate from the repo root:
#   cargo build --release -p mnem-cli
#   vhs assets/demo.tape

Output assets/demo.gif

Set Shell bash
Set FontSize 15
Set Width 1080
Set Height 500
Set Padding 24
Set Theme "Dracula"
Set TypingSpeed 40ms

Hide
Type `export PATH="$PWD/target/release:$PATH"; export PS1="$ "; cd "$(mktemp -d)"; clear`
Enter
Sleep 400ms
Show

Type "mnem init"
Enter
Sleep 1s

Type "# an agent records what it learns, with where it came from"
Enter
Type `mnem add customer-4821 "on the Enterprise plan" --source ticket-4821`
Enter
Sleep 700ms
Type `mnem add case-4821-status "triaging"`
Enter
Sleep 900ms

Type "# freeze the current memory as a commit"
Enter
Type `mnem commit -m "open the support case" --author support-agent`
Enter
Sleep 1200ms

Type "# the belief changes later in the run"
Enter
Type `mnem add customer-4821 "downgraded to Pro" --source ticket-4821`
Enter
Sleep 700ms
Type `mnem commit -m "record the downgrade" --author support-agent`
Enter
Sleep 1200ms

Type "# the whole history, newest first"
Enter
Type "mnem log --oneline"
Enter
Sleep 3500ms
