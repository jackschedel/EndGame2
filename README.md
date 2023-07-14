<h1 align="center"><b>EndGame2</b></h1>

<h4 align="center"><b>UCI-Compatible Chess Engine Using Minimax Tree Traversal</b></h4>

<a href="https://github.com/jackschedel/EndGame2/issues" target="_blank">
<img src="https://img.shields.io/github/issues/jackschedel/EndGame2?style=flat-square" alt="issues"/>
</a>
<a href="https://github.com/jackschedel/EndGame2/pulls" target="_blank">
<img src="https://img.shields.io/github/issues-pr/jackschedel/EndGame2?style=flat-square" alt="pull-requests"/>
</a>

## 🌐 Overview

EndGame2 is a UCI-compatible chess engine developed in Rust that uses a minimax tree-traversal approach to evaluate and decide chess moves. It incorporates alpha-beta pruning, multithreading, and position hashing for improved performance. The project is a personal initiative to learn Rust and is actively being developed and expanded.

## ⚙️ Features

- **UCI-Compatible Engine:** EndGame2 is compatible with the Universal Chess Interface (UCI), ensuring its broad usability with various chess GUIs.
- **Minimax Tree Traversal:** The engine uses the minimax algorithm to evaluate board states and make optimal decisions.
- **Alpha-Beta Pruning:** EndGame2 implements alpha-beta pruning to reduce the number of nodes evaluated by the minimax algorithm, thereby increasing efficiency.
- **Multithreading:** The engine utilizes multithreading to enhance performance by allowing multiple operations to be carried out simultaneously.
- **Position Hashing:** EndGame2 employs position hashing to save and retrieve previously analyzed board states, further boosting performance.
- **Written in Rust:** The engine leverages the power and finesse of the Rust programming language for its development.

## 💡 About

EndGame2 starts by interfacing with the UCI protocol, allowing the engine to communicate with any chess GUI supporting UCI. It then uses the minimax algorithm to analyze the current board state, going through all possible move combinations to identify the optimal move. 

The engine employs alpha-beta pruning to reduce the number of nodes it needs to evaluate, thus improving efficiency. It also uses multithreading to allow multiple operations to happen simultaneously, enhancing the speed of the engine. 

To further improve performance, EndGame2 uses position hashing to store and retrieve analyzed board states, reducing the need for repeated analysis of the same positions. 

The project is a personal initiative to learn Rust, and its development is ongoing, with new features and improvements being continually added.
