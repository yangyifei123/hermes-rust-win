Set FontSize 14
Set Width 1200
Set Height 600
Set Padding 10

Type "cargo install hermes-agent-cli"
Enter
Sleep 2

Type "hermes auth add openai sk-demo-key"
Enter
Sleep 1

Type "hermes models --provider openai"
Enter
Sleep 1

Type "hermes chat --provider openai --model gpt-4o --quiet"
Enter
Sleep 1

Type "Write a Rust function that reverses a string"
Enter
Sleep 3

Type "/compact"
Enter
Sleep 1

Type "/model deepseek-chat"
Enter
Sleep 1

Type "Now make it handle Unicode"
Enter
Sleep 3

Type "/quit"
Enter
Sleep 1
