<html>
  <head>
    <meta content="text/html;charset=utf-8" http-equiv="Content-Type" />
    <link rel="stylesheet" type="text/css" href="../css/minimal.css">
  </head>
</html>

# Dungeon Crawler
This is a dungeon crawler rogue-like game written in Rust and compiled to WebAssembly. Playing the game requires a keyboard to play.
The project is from [Hands-on Rust](https://www.amazon.com/Hands-Rust-Effective-Learning-Development/dp/1680508164/) by Herbert Wolverson.

# Game Premise
You are an adventurer. Your quest is to find the Amulet of Yala. You must navigate three maze-like levels filled with evil monsters to find it. You will find helpful items along the way. Death is imminent.

# Instructions
- Arrow or 'W','S','A','D' keys to move
- 'g' key to pick up an item
- Number keys to use an item
- '.' key to skip a turn

<html>
  <body style="width: 800px;">
    <canvas id="canvas" width="640" height="480"></canvas>
    <script src="./dungeoncrawl.js"></script>
    <script>
      window.addEventListener("load", async () => {
        await wasm_bindgen("./dungeoncrawl_bg.wasm");
      });
    </script>
  </body>
</html>

