<html>
  <head>
    <meta content="text/html;charset=utf-8" http-equiv="Content-Type" />
    <link rel="stylesheet" href="../css/minimal.css">
  </head>
</html>

# Dungeon Crawler
This is a dungeon crawler rogue-like game.
It is written in Rust and deployed to WebAssembly.
The project is from [Hands-on Rust](https://www.amazon.com/Hands-Rust-Effective-Learning-Development/dp/1680508164/) by Herbert Wolverson.
Playing the game requires a keyboard to play.

# Game Premise
You are an adventurer. Your quest is to find the Amulet of Yala.
You must navigate the three maze-like levels filled with evil monsters to find it.
You will find helpful items along the way. Death is imminent.

# Instructions
Use the arrow keys to move.
Use the 'g' key to pick up an item.
Use the number keys to use an item.
There are three levels.

<html>
  <body>
    <canvas id="canvas" width="700" height="500"></canvas>
    <script src="./dungeoncrawl.js"></script>
    <script>
      window.addEventListener("load", async () => {
        await wasm_bindgen("./dungeoncrawl_bg.wasm");
      });
    </script>
  </body>
</html>

