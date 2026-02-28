# WhizCode - Local AI Agentic IDE

WhizCode is a local AI agent desktop application, styled to look and feel like Visual Studio Code. It provides a familiar interface for developers to interact with their local AI assistant (Ollama).

Built with React, TypeScript, Vite, and Electron.

## Features

- **VS Code Aesthetic**: A dark theme UI that closely mirrors Visual Studio Code, complete with an Activity Bar, Sidebar, Editor Tabs, and a Terminal-like chat interface.
- **Local AI Integration**: Designed to integrate with your local AI agent processes via Electron IPC.
- **Fast Development**: Powered by Vite for lightning-fast hot module replacement (HMR).

## Getting Started

### Prerequisites

Ensure you have the following installed on your machine:

- [Node.js](https://nodejs.org/) (v18 or newer recommended)
- [npm](https://www.npmjs.com/) (comes with Node.js)

### Installation

1. Clone or download the repository to your local machine.
2. Open a terminal and navigate to the project directory:

   ```bash
   cd path/to/WhizCode
   ```

3. Install the dependencies:

   ```bash
   npm install
   ```

## How to Execute

### Development Mode

To start the application in development mode with hot-reloading:

```bash
npm run dev
```

This will concurrently:
- Spin up the Vite dev server for the React frontend.
- Launch the Electron application window.

Any changes you make to the React components or styles will automatically reflect in the application without needing a full reload.

### Building for Production

To create a production build of the application:

```bash
npm run build
```

This processes the TypeScript files and builds the optimized bundles for both the Vite frontend and the Electron main process.

## Project Structure

- `src/` - Contains the React frontend code (Components, CSS).
- `electron/` - Contains the Electron main process code and IPC handlers.
- `public/` - Static assets.
- `index.html` - The main HTML entry point for the Vite application.
- `vite.config.ts` - Vite configuration, including Electron plugin setup.
