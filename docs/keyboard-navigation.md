# Navigate the workspace with the keyboard

Available in **0.11.1**. The drawing format remains v7 and the agent API remains v1.

## Move between areas

Press **F6** to move through the ribbon, library, drawing canvas, properties and
page bar. **Shift+F6** follows the reverse order. When no interface control has
focus, navigation starts from the canvas. On Mac keyboards that assign system
actions to function keys, hold **Fn** as well.

A dark blue outline identifies the focused control or canvas. The status bar
names the destination. Hidden side panels remain reachable through their toggle;
moving focus does not open them or change your preferred layout.

Within an area, **Tab / Shift+Tab** follow the interface controls. **Enter**
activates the focused button. A panel toggle keeps focus when it opens or closes
the panel. The command palette and F1 help also list the two area-navigation
commands.

## Create and edit a block

1. Move to the library using F6 or Shift+F6.
2. Use Tab to reach a named shape and press Enter to choose it.
3. Press F6 to reach the canvas, then Enter to place the shape in the current view.
4. Type directly inside the block. Press Ctrl+Enter / Cmd+Enter to finish, or
   Escape to cancel the text edit.
5. With the canvas focused, use arrow keys for 1 px movements or Shift+arrows
   for 10 px. F2 or Enter edits a selected block. Ctrl+S / Cmd+S saves it.

To select another existing object without a pointer, use Ctrl+F / Cmd+F to search
by text, type or ID. The [object search guide](recherche-objets.md) describes
selection, additive selection and connecting two blocks.

F6 does not leave an inline text edit, a modal dialog or an active pointer gesture.
Finish or cancel that operation first. Text fields retain their arrow keys and
editing shortcuts; Enter on an interface button does not edit a block behind it.
Moving between areas preserves the current drawing selection and undo history.

## Accessibility validation

The canvas, panel toggles, symbol tiles and common icon buttons expose names in
the selected interface language. The document close button includes its title.
Automated tests inspect these accessibility-tree names and keyboard focus in all
seven languages at 1200 × 760 points.

These checks cover the underlying interface tree and simulated input. Human
screen-reader tests, operating-system announcements, contrast across all states
and the complete workflow on a 13-inch MacBook remain part of the
[1.0 quality work](quality.md). This version does not claim complete screen-reader
support.
