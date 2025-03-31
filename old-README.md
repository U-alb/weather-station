[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-24ddc0f5d75046c5622901739e7c5dd533143b0c8e959d652212380cedb1ea36.svg)](https://classroom.github.com/a/l2EJ6P3t)
# Maze Solver

## Description

A device that can solve mazes (yes, folks, it does what it says on the can).

## Hardware

<!-- Fill out this table with all the hardware components that you mght need.

The format is 
```
| [Device](link://to/device) | This is used ... | [price](link://to/store) |

```

-->

| Device | Usage | Price |
|--------|--------|-------|
| [Rapspberry Pi Pico W](https://www.raspberrypi.com/documentation/microcontrollers/raspberry-pi-pico.html) | The microcontroller | [35 RON](https://www.optimusdigital.ro/en/raspberry-pi-boards/12394-raspberry-pi-pico-w.html) |
| [Wires](https://www.smart-prototyping.com/image/cache/data/2_components/others/cables/breadboard-cables-40-x-100mm-male-to-female-2-54mm-44290-600x315.jpg) | This is used for connections| |
| [LCD Display]() | This is optional, used for displaying the route| [60 RON](https://www.optimusdigital.ro/ro/lcd-uri/12673-display-lcd-de-13-pentru-raspberry-pi-pico-cu-65k-culori-240x240-spi.html) |

## What got me the idea

A lecture regarding binary search trees

## Implementation Quirks
The basic idea is using the Wifi of the Pico by uploading scanned images or documents of mazes on a web interface, which it will process and then compute the solution for.

It will optically detect the walls, transform them into a Binary Search Tree and compute the minimal path (which actually is the solution to the maze).

I'm also thinking of using a display, so that the Pico can also graphically display the solution instead of just sending a trace back to the client through the web app. 
In my conception, possible solutions (in the order of difficutly) would be a string consisting in the characters L (left) and R (right), a string consisting of the route (for example 1 9 12 4 7 20 3, where each number is a node), or directly an image of the route overlayed over the original document.
