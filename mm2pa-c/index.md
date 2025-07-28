<html lang="en">
<head>
    <!-- Google tag (gtag.js) -->
    <script async src="https://www.googletagmanager.com/gtag/js?id=G-LYQJQNJ4PW"></script>
    <script>
        window.dataLayer = window.dataLayer || [];
        function gtag(){dataLayer.push(arguments);}
        gtag('js', new Date());

        gtag('config', 'G-LYQJQNJ4PW');
    </script>

    <meta charset="utf-8">
    <link rel="icon" href="favicon.ico" type="image/x-icon">
    <title>MegaMan 2: Puzzle Attack - 🦀 Carcinized</title>
    <style>
        html,
        body {
            /* Body takes 100% width, 100% height, centered */
            width: 100%;
            height: 100%; /* Use 100% viewport height */
            margin: 0 auto; /* Center body horizontally */
            padding: 0; /* Remove default padding */
            overflow: hidden; /* Prevent scrollbars on body */
            background: white;
            position: relative; /* Needed for absolute positioning of canvas */
        }

        canvas {
            display: block;
            margin: 0;
            padding: 0;
            overflow: hidden;
            position: absolute;
            background-color: black; /* Set canvas background */
            border: 0; /* Ensure no border affects layout */
            z-index: 0;
            /* Sizing and positioning handled by JS */
            /* top: 10px; will be set by JS */
            /* left: ...; will be set by JS */
        }
    </style>
</head>
</html>

# MegaMan 2: Puzzle Attack - 🦀 Carcinized
MegaMan 2: Puzzle Attack - 🦀 Carcinized is a Tetris / MegaMan 2 mashup fan game. It is a Rust-port based on [MegaMan 2: Puzzle Attack](https://github.com/markpulver/mega-man-2-puzzle-attack) written in Python by [Mark Pulver](https://markpulver.com) in 2005.

***

<html>
<body>
    <canvas id="glcanvas" tabindex='1'></canvas>
    <!-- Minified and statically hosted version of https://github.com/not-fl3/macroquad/blob/master/js/mq_js_bundle.js -->
    <!-- <script src="https://not-fl3.github.io/miniquad-samples/mq_js_bundle.js"></script> -->
    <script src="mq_js_bundle.js"></script>
    <script src="sapp_jsutils.js"></script>
    <script src="quad-storage.js"></script>

    <script>
        const canvasElement = document.getElementById('glcanvas');
        const bodyElement = document.body; // Get reference to body
        const aspectRatio = 3 / 2; // Width / Height ratio

        function resizeAndCenterCanvas() {
            // --- Calculate Available Space (97% of body) ---
            // Use clientWidth/clientHeight which account for padding but not borders/margins
            const availableWidth = bodyElement.clientWidth * 1.;
            const availableHeight = bodyElement.clientHeight * 0.97;

            // --- Determine Target Size based on Aspect Ratio and Available Space ---
            let targetWidth, targetHeight;

            // Calculate size if available width is the limiting factor
            let widthBasedOnWidth = availableWidth;
            let heightBasedOnWidth = widthBasedOnWidth / aspectRatio;

            // Calculate size if available height is the limiting factor
            let heightBasedOnHeight = availableHeight;
            let widthBasedOnHeight = heightBasedOnHeight * aspectRatio;

            // Choose the scenario that fits within *both* available dimensions
            // If sizing based on width makes it too tall, then we must size based on height.
            if (heightBasedOnWidth <= availableHeight) {
                // Width-based calculation fits vertically, use it
                targetWidth = widthBasedOnWidth;
                targetHeight = heightBasedOnWidth;
            } else {
                // Width-based calculation was too tall, so height must be the constraint
                targetWidth = widthBasedOnHeight;
                targetHeight = heightBasedOnHeight;
            }

            // Ensure minimum size (optional, but good practice)
            targetWidth = Math.max(320, targetWidth); // Min width 10px
            targetHeight = Math.max(240 / aspectRatio, targetHeight); // Min height based on ratio

            // Round to whole pixels
            targetWidth = Math.round(targetWidth);
            targetHeight = Math.round(targetHeight);

            // --- Apply Size to Canvas ---
            canvasElement.width = targetWidth; // Set drawing buffer size
            canvasElement.height = targetHeight;
            canvasElement.style.width = targetWidth + 'px'; // Set CSS display size
            canvasElement.style.height = targetHeight + 'px';

            // --- Center Canvas Horizontally within Window ---
            // Note: Centering within the window, not the body element
            const windowWidth = window.innerWidth;
            const leftOffset = (windowWidth - targetWidth) / 2;
            canvasElement.style.left = Math.round(leftOffset) + 'px';

            // --- Set Vertical Position ---
            canvasElement.style.top = '10px'; // Keep the 10px top offset
        }

        // Run resize logic initially and on window resize
        window.addEventListener('DOMContentLoaded', resizeAndCenterCanvas);
        window.addEventListener('resize', resizeAndCenterCanvas);

        // Load the Wasm module
        load("rustman.wasm");

        // A small delay after load helps when wasm init interferes
        setTimeout(resizeAndCenterCanvas, 500);

    </script>
</body>
</html>

***

# Playing
⚠️ As of v0.1 on 2025-06-21, touch screen controls have not been implemented so your device must have a keyboard to play.

⌨️ Controls
Primary Controls use Arrow Keys, WASD-style keys, or the Number Pad
- Arrow keys:  Move Left/Right: ⬆️/⬇️, Move Down: ⬇️, Rotate Clockwise: ⬆️
- WASD keys:  Move Left/Right: A / D, Move Down: X or S, Rotate Anti-/Clockwise: Q / E
- Number Pad: Move Left/Right: 4 / 6, Move Down: 2 or 5, Rotate Anti-/Clockwise: 8 or 9 / 7

Secondary Controls:
- Instant Drop: SpaceBar or Numpad 0 
- Switch Active Power: F1 - F9 or the number row: 1 - 9
- Pause Game: Enter or NumPad Enter
- Quit Game:  Escape key

🥇 Tips
- Complete Tetris lines to do damage against robots
- Acquire power-ups after defeating robots
- Activate power-ups to do bonus damage against robots
- Each Robot has different weaknesses to each of the different power-ups
- Some robots have immunity against certain power-ups
- Defeat the 8 robots to enable Dr. Wily's stage
- The Game Timer tracks how long it takes to defeat robots. Go fast to earn your place on the Best Times list