# Recorded by [vhs](https://github.com/charmbracelet/vhs)
# Where should we write the GIF?
Output demo.gif

# Set up a 1200x600 terminal with 42px font.
Set FontSize 20
Set Width 1200
Set Height 800

# ## Start clashtui
Type "clashtui"
Sleep 1s
Enter
Sleep 1s

# ## Help
Set TypingSpeed 1s
Type "?"
Type "jjjj"
Escape

# ## Profile
Type "i"
Set TypingSpeed 300ms
Type "sub"
Tab
Type "https://example.com"
Enter

Set TypingSpeed 1s

# scroll list
Type "j"
Sleep 2s
Type "k"
Sleep 2s

# Select and update the profile
Enter
Sleep 5s
Type "u"
Sleep 10s
Escape

# Preview the profile
#Type "v"
#Sleep 2s
#Escape

# Generate profile with Template
Type "t"
Sleep 1s
Enter
Sleep 1s
Escape
Sleep 1s
Type "p"
Sleep 1s

# Delete profile
Type "d"
Sleep 2s
Type "y"
Sleep 1s

# ## ClashSrvCtl
Set TypingSpeed 300ms
Type "2"
Type "jjjj"
Type "j"

# ## Global
# Show log
Type "L"
Sleep 2s
Escape

# ## Exit
Type "q"
Sleep 1s
