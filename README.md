# HotKeys
A simple Windows app for mapping keyboard shortcuts/actions to numeric keypad keys [1-9]. It's built primarily for software developers, but can be used in the same by any Windows user that prefers keyboard over a mouse to reach most frequently used commands in aplications of their choice.
- [HotKeys](#hotkeys)
- [Usage](#usage)
- [Configuration](#configuration)
  - [Profile and Pads](#profile-and-pads)
  - [Shortcut action](#shortcut-action)
  - [Text/Line action](#textline-action)
  - [Pause action](#pause-action)
  - [Board action](#board-action)
  - [Other settings](#other-settings)
- [Known issues (TBD)](#known-issues-tbd)

# Usage
Once started, application runs silently in the background until shutdown from system tray icon menu, or until triggered by a dedicated global keyboard shortcut. It is used in the following way:

- User hits `Ctrl Alt NumPad_0`
- HotKeys wakes up and finds the app currently running in foreground
- If the user has any mappings configured for the foreground app:
  - Board with 3x3 is displayed
  - Users can select & apply an action using numeric keys [1-9]
    - or close the board by pressing any other key
    - or wait few seconds until the board closes by itself

# Configuration
Application uses two configuration files locaded in *resources* folder:
- settings.json
- log.toml

Users can configure general application settings by editing the
[settings.json](resources/settings.json) file. These settings include color schemes, keyboard layouts, board auto-close timeout, etc... The file is also used to configure mappings/actions for specific applications. Maximum number of mappings per appliction is 9 (3x3 board), and there is no limit on the number of aplications. If a user wants to map more than 9 actions for an app, it's possible to organize boards hierarchically.
Model file contains a list of **Profile** entries. One profile defines the mappings configured for one application. A profile may have up to nine **Pads**, where each pad defines a sequence of **Actions** mapped to one numeric key. Available action types are shown in table below.

| # | Action | Description |
| - | ---------| -----------------------------------|
| 1 | Shortcut | Sends keyboard shortcut to the app |
| 2 | Text | Sends arbitrary text to the app |
| 3 | Line | Same as Text, but appends ENTER to the text |
| 4 | Pause | Waits *n* milliseconds |
| 5 | Board | Opens another board (e.g. open sub-board or back to parent board) |

File [log.toml](resources/log.toml) is used to configure application logging.

## Profile and Pads
An example of Profile configuration for *InteliJ*:
```json
{
  "keyword": "idea64",
  "name": "InteliJ",
  "color_scheme": "Violet",
  "pads": [
    {
      "description": "Show in Projects",
      "title":"Alt F1 + 1",
      "actions": [ { "Shortcut" : "Alt F1 + 1" } ]
    }
  ]
}
```
Parameter `keyword` is used to match the foreground application with a given profile. If the application name contains the profile keyword (case-insensitive), the match is successfull, and the board for that profile will be displayed. Usage of parameter `color_scheme` is optional, if not set the main application color scheme is used. Parameter `name` is used as board title.

Pad configuration includes the `title` and `description` (tile header and content), both of which may be ommited, and the list of actions.

## Shortcut action
This action can have one or more key-combinations, separated by a `+` sign. Few examples:
```
Ctrl Shift P
Alt F
Ctrl K + Ctrl B
```
Shorcut definitions are case insinsitive and extra whitespaces are ignored. All combinations below are considered equal:
```
ctrl alt p
Ctrl Alt P
CTRL ALT P
ctrl alt 'P'
cTrL aLt  P
```
Letters, numbers, interpunction and other symbols can be used with or without the single-quotes. Only exception is a `+` sign, when used as part of the shortcut it has to be wrapped in single-quotes. When used to connect two key-combinations it is used without quotes. Following expresions are equal and both are valid.
```
Ctrl Shift '-' + Ctrl Shift '+'
Ctrl Shift - + Ctrl Shift '+'
```
See [Virtual-Key codes](docs/virtual_keys.md) for full list of symbols and key-codes available.
## Text/Line action
These actions can be used whenever an arbitrary text needs to be sent to an application. Consider the following example for Chrome browser:
```json
{
  "description": "My Github",
  "title":"",
  "actions": [
    { "Shortcut" : "Ctrl T" },
    { "Line" : "https://github.com/ivicakukic" }
  ]
}
```
First action will open a new tab. The URL sent by the second action is will be put into the address bar (the browser focuses on the address bar after opening the new tab) and the additional ENTER key appended by the Line action will tell the browser to navigate to the URL.

## Pause action
Can be used whenever the target application UI needs some time to execute the previous action. The sleep time is defined in milliseconds. Next example shows how to activate '**P**roject -> Quick **S**witch Project" menu item in *Sublime Text*. The example wouldnt work without a pause, *Sublime* needs some ~200 ms to open the first menu item.
```json
{
  "description": "Switch Project",
  "title":"Alt P + S",
  "actions": [
    { "Shortcut" : "Alt P" },
    { "Pause" : 500 },
    { "Shortcut" : "S" }
  ]
}
```

## Board action
Can be used for creating board hierarchies. Next example shows how to configure an action that opens a *VS Code*  sub-board having a keyword 'code/run'.
```json
{
  "description": "RUN",
  "actions": [ { "Board" : "code/run" } ]
}
```
Same principle can be used on sub-board to return back to the main the main *VS Code* board:
```json
{
  "description": "<<",
  "actions": [ { "Board" : "code" } ]
}
```
## Other settings
Application settings example with parameters explained.
```json
{
  // time after which the board closes automatically (sec)
  "timeout": 4.0,

  // pad selection visual feedback duration (ms, 0 - no feedback)
  "feedback": 0.05,

  // txt editor to open configuration files
  "editor": "notepad.exe",

  // available color schemes
  "color_schemes" : [
    {
      "name":  "Blue",
      "opacity": 0.8,           // board opacity
      "background": "#00007f",  // board color
      "foreground1": "#5454a9", // font folor
      "foreground2": "#dbdbec"  // line color
    }
  ],

  // default and available keyboard keys layout mappings, optional
  "keyboard_layout": "croatian",
  "keyboard_layouts" : [
      {
          "name" : "croatian",
          // "x":"y" (x = user's language keyboard; y = english keyboard)
          "mappings":{
              "š":"[",
              "Š":"{",
              "đ":"]"
          }
      }
  ],

  // profiles (see examples above)
  "profiles": []
}
```

# Known issues (TBD)

- [x] VK codes
- [ ] Board not on top
- [ ] Hook lost for one targer
