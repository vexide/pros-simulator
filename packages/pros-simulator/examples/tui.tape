# This is a vhs script. See https://github.com/charmbracelet/vhs for more info.
# To run this script, install vhs and run `vhs ./examples/tui.tape`
Require "bat"
Require "cargo"
Output "target/tui.gif"
Set Width 2000
Set Height 800
Sleep 1s
Type "bat -pp ./example/src/main.rs"
Enter
Sleep 3s
Type "cargo run --example=tui"
Enter
Sleep 13s
Type "q"
Sleep 1s
