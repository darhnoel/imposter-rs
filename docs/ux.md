# UX: Web Frontend Flow

## Main Interactions
- Create room: admin nickname + room code.
- Join room: room code + nickname.
- Leave room: exits current session.
- Admin actions: set category, start game, next turn, restart game.
- Guess action: any connected player can submit `guessImposter`.
- Role card: hidden by default, reveal/hide toggle in UI.

## Screen Behavior

### Lobby and Players
- Shows room code and connected players list.
- Admin and players join from same lobby panel.
- Disconnected players are excluded from visible player list.

### Game Panel
- Shows current turn player id.
- Allows guess submission and manual snapshot refresh.

### Result Panel
- Shows winner, guessed player id, and imposter player id after round end.

## Role Card Rules

Hidden:
- Default state on load and after leave.
- Displays blocked placeholder.

Revealed:
- CREW: shows role type and topic id.
- IMPOSTER: shows imposter message.
