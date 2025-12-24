# **fero**

fero is a lightweight, state-driven terminal text editor built in rust.

### **a "stepping stone" editor**

fero was designed specifically to bridge the gap between basic editors like **NANO** and complex modal editors like **VIM**. it provides the approachability of a simple editor—using intuitive menus and familiar navigation—while introducing "serious" power-user features like multi-buffer management, a recursive file explorer, and a live-updating theme engine.

## **QUICK START**

IMPORTANT  
NAVIGATION TIP: fero uses a non-modal management system. press ESC at any time to open the MAIN MENU. from there, you can navigate the file explorer, adjust settings, or open the live palette editor without memorizing complex shortcuts. you will have to use the config menu under fero if you would like to setup the shortcuts, there are many common ones preconfigured but not set standard as to not mess with your typical terminal workflow. the rebind process is quick and easy and allows you to use the shortcuts you want and not have to undo the ones you dont. 

### **prerequisites**

* rust & cargo (latest stable)
* a terminal supporting 24-bit color (alacritty, iterm2, kitty, etc.)

### **installation**

as it is still in early developement. fero is currently built from source. 
follow these steps to get up and running:

1. **CLONE THE REPOSITORY**:  
   git clone \[https://github.com/travtherobber/fero.git\](https://github.com/travtherobber/fero.git)  
   cd fero

2. **BUILD THE OPTIMIZED RELEASE BINARY**:  
   cargo build \--release

3. **RUN FERO**:  
   ./target/release/fero \[filename\]

## **OPTIONAL: SETUP A ZSH ALIAS**

to use fero from any directory without typing the full path, add an alias to your shell configuration.

1. open your .zshrc file:  
   nano \~/.zshrc

2. add this line at the bottom (replace /path/to/fero with the actual path to your cloned repo):  
   \# fero text editor  
   alias fero='/path/to/fero/target/release/fero'

3. source the config to apply changes:  
   source \~/.zshrc

## **FEATURES**

* **STATE-DRIVEN ARCHITECTURE**: transitions seamlessly between editing, file browsing, and configuration using a central appstate machine.  
* **LIVE THEME ENGINE**: modify ui colors (background, accents, panels) in real-time. changes are serialized to config.toml instantly.  
* **INTEGRATED EXPLORER**: a recursive directory tree that allows you to find and inject files into new buffers without leaving the app.  
* **MULTI-BUFFER LOGIC**: edit multiple files simultaneously with independent cursor tracking and undo/redo history.  
* **CUSTOM SYNTAX HIGHLIGHTING**: fast, regex-based highlighting engine for rust, python, and bash.  
* **SYSTEM CLIPBOARD**: full integration with the system-level copy/paste via arboard.

# **fero: technical architecture & implementation**

## **1\. STATE-DRIVEN UI ARCHITECTURE**

fero avoids the "spaghetti code" common in early terminal projects by utilizing a centralized state machine defined in state.rs.

### **context-aware input handling**

the application uses a Mode enum to dictate how the system responds to user intent.

* **STATE SEPARATION**: the logic for Mode::Editing is physically and logically separated from Mode::Explorer or Mode::ColorEditor.  
* **TRANSITION LOGIC**: transitions are triggered by specific events (e.g., pressing ESC to move from Editing to Menu). this ensures that the ui never enters an "invalid" state where a user is accidentally typing text into a file explorer.

### **central appstate**

all mutable data—including the list of open Buffer objects, the current Palette, and the Keybind map—resides in a single AppState struct. this allows for easy serialization (saving settings) and makes the entire app's state easy to reason about and debug.

## **2\. BUFFER & VIEWPORT MANAGEMENT**

handling text files that are larger than the terminal screen requires a robust coordinate system. fero implements a custom viewport logic to handle this.

### **coordinate system**

the Buffer struct tracks two sets of coordinates:

1. **CURSOR COORDINATES** (cursor\_x, cursor\_y): the absolute position of the cursor within the text file.  
2. **VIEWPORT OFFSETS** (viewport\_offset\_x, viewport\_offset\_y): the "camera" position, representing the top-left character currently visible on the screen.

### **adaptive scrolling**

the scroll\_to\_cursor function in state.rs calculates whether the cursor has moved outside the visible bounds. if it has, the viewport offsets are updated to "pull" the camera along. this results in a smooth scrolling experience that supports both horizontal and vertical movement.

## **3\. LIVE THEME ENGINE & PERSISTENCE**

one of fero's standout features is its ability to modify its own appearance without a restart.

### **toml serialization**

using serde and toml, fero maps its internal Palette struct directly to a config.toml file.

* **DATA FLOW**: user changes a hex code in the ColorEditor \-\> AppState.current\_palette is updated \-\> config::save\_config writes the changes to disk.  
* **INSTANT FEEDBACK**: because the redraw\_all loop uses the current\_palette on every tick, the user sees the color change the exact millisecond they hit enter.

## **4\. RENDER PIPELINE**

fero utilizes crossterm for low-level terminal manipulation, following a "dirty-rect" style of thinking (though adapted for terminal character cells).

### **low-latency redrawing**

the ui.rs module is responsible for translating the AppState into ansi escape codes.

* **QUEUEING**: instead of writing to stdout for every character, fero uses queue\! to batch instructions.  
* **FLUSHING**: a single stdout.flush() call at the end of the render loop ensures all changes are pushed to the terminal at once, preventing the "flicker" effect common in naive tui implementations.

## **5\. SYNTAX HIGHLIGHTING ENGINE**

fero implements a high-speed, regex-based tokenization engine.

* **LAZY LOADING**: language keywords (rust, python, bash) are stored in LazyLock hashsets to ensure they are only initialized once, saving memory and startup time.  
* **TOKEN SCANNING**: as the ui renders each line, it scans for strings, comments, and keywords, applying SetForegroundColor instructions dynamically.

## **ROADMAP**

+ UX improvements: streamlining menu navigation, adding smoother transition animations, and improving the responsiveness of the file explorer.
+ architectural overhaul: making the core code more modular with less circular calls.
+ advancing the syntax engine: moving beyond regex-based highlighting to implement a more context-aware system (likely via tree-sitter) for deeper language support.
+ search and replace: the feature is currently broken and not worth fixing until after the overhaul.
+ crates.io release: reaching a stable v0.1.0 for distribution via the rust package manager.
+ 
## **CONTRIBUTING**

contributions are what make the open-source community an amazing place to learn, inspire, and create. feel free to open a pull request.

## **CONTACT & SUPPORT**

* **AUTHOR**: TravTheRobber  
* **EMAIL**: anguishedkitty @ proton (dot) me
* **GITHUB**: [github.com/travtherobber](https://www.google.com/search?q=https://github.com/travtherobber)

if you need further documentation for modifying fero or fixing it just send me an email and ill have it to you as soon as i get it.

* if you like fero and would like to support me $TravTheRobber (chime)


**BUILT WITH RUST FOR THE NEXT GENERATION OF TERMINAL USERS.**
