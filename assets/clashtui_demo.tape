# Recorded by [vhs](https://github.com/charmbracelet/vhs)
# Where should we write the GIF?
Output demo.gif

# Set up a 1200x600 terminal with 46px font.
Set FontSize 20
Set Width 1200
Set Height 1000

Type "./target/release/clashtui"
Sleep 500ms
Enter
Sleep 1s

Set TypingSpeed 1s

# ## Close Welcome
Escape 

# ## Help
Type "?"
Type "jjjj"
Escape

# ## Profile
Type "i"
Set TypingSpeed 300ms
Type "sub"
Tab
Type "the url"
Escape

Set TypingSpeed 1s

Enter
Type "j"
Enter
Type "k"

Type "u"
Sleep 10s
Escape
Type "P"
Sleep 1s
Escape
Type "L"
Sleep 1s
Escape

Type "t"
Sleep 1s
Enter
Sleep 1s
Escape
Sleep 1s
Type "p"
Enter
Sleep 6s
Escape
Sleep 1s
Type "D"
Sleep 1s
Type "y"
Sleep 1s

Type "P"
Sleep 2s
Escape
Sleep 2s

# ## ClashSrvCtl
Set TypingSpeed 300ms
Type "2"
Type "jjjj"
Enter
Type "j"
Enter

# ## Config
Tab
Enter
Type "the path to clash config dir"
Enter
Sleep 1s
Enter
Escape

# ## Global
Type "H"
Sleep 500ms

# ## Exit And Rerun
Type "Q"
Sleep 1s
Type "./target/release/clashtui"
Sleep 500ms
Escape