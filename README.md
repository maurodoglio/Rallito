# Rallito 🎾

A web application for padel players to track game results and generate leaderboards.

## Features

- **Player Management**: Add and manage players
- **Game Recording**: Record match results with team scores
- **Leaderboard**: Automatic ranking based on points and wins
- **Game History**: View all recorded games
- **Points System**: Winners earn 3 points per game

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo

### Installation

1. Clone the repository:
```bash
git clone https://github.com/maurodoglio/Rallito.git
cd Rallito
```

2. Build the project:
```bash
cargo build --release
```

3. Run the application:
```bash
cargo run --release
```

4. Open your browser and navigate to:
```
http://localhost:3000
```

## Usage

### Adding Players

1. Go to the "Players" tab
2. Enter a player name and click "Add Player"

### Recording a Game

1. Go to the "Record Game" tab
2. Select 4 different players (2 per team)
3. Enter the scores for each team
4. Click "Submit Game"

The system will automatically:
- Update player statistics (wins/losses)
- Award 3 points to each winner
- Update the leaderboard

### Viewing the Leaderboard

The leaderboard automatically ranks players by:
1. Points (3 per win)
2. Total wins
3. Alphabetically by name

## Technical Details

- **Backend**: Rust with Axum web framework
- **Database**: SQLite
- **Frontend**: Vanilla HTML/CSS/JavaScript

## Development

Run in development mode:
```bash
cargo run
```

The application will be available at `http://localhost:3000`

## License

See LICENSE file for details.
