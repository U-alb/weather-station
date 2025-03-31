# Weather Station

Rust-powered environmental monitoring station

## Description

The project builds a weather station which provides real-time weather data on an external display and web interface

## Motivation

Building a Raspberry Pi Pico W weather station with embassy-rs combined my interest in embedded systems with a chance to explore networking. It allows me to learn sensor interaction and data processing. Integrating a web interface adds a network layer, enabling real-time data display and opening doors for future functionalities like remote monitoring.


## Hardware

We will use a BME280 in order to gather environmental data. The Pico will process this gathered data in order to print meaningful graphics on the provided display. We will also use several push buttons in order to switch between the available data


### Bill of Materials

| Device | Usage | Price |
|--------|--------|-------|
| [Raspberry Pi Pico WH](https://www.raspberrypi.com/documentation/microcontrollers/raspberry-pi-pico.html) | The microcontroller | [56 RON](https://ardushop.ro/ro/home/2819-raspberry-pi-pico-wh.html) |
| [BME280](https://www.bosch-sensortec.com/products/environmental-sensors/humidity-sensors-bme280/) | Temperature, pressure and humidity sensor | [74 RON](https://www.optimusdigital.ro/ro/senzori-senzori-de-presiune/5649-modul-senzor-barometric-de-presiune-bme280.html) |
| [LED Display](https://ardushop.ro/8014-thickbox_default/modul-lcd-spi-128x160.webp) | Display | [39 RON](https://ardushop.ro/ro/home/2818-modul-lcd-spi-128x160.html) |
| [Breadboard](https://www.yamanelectronics.com/wp-content/uploads/2020/06/basics-of-breadboard.webp) | The physical base of the project | [10 RON](https://www.optimusdigital.ro/ro/prototipare-breadboard-uri/8-breadboard-830-points.html) |
| [Jumper Wires](http://www.atomsindustries.com/assets/images/items/1075/1075.webp) | For connecting all the different components | [7 RON](https://www.optimusdigital.ro/ro/fire-fire-mufate/886-set-fire-tata-tata-40p-15-cm.html) |
| [Push buttons](https://ardushop.ro/655-thickbox_default/buton-mic-push-button-trough-hole.webp) | For switching info on the screen| [0.36 RON each / 1.5 RON total](https://www.optimusdigital.ro/ro/butoane-i-comutatoare/1119-buton-6x6x6.html)
 

## Software

| Library | Description | Usage |
|---------|-------------|-------|
| [embassy-rp](https://github.com/embassy-rs/embassy/tree/main/embassy-rp) | The embassy-rp HAL targets the Raspberry Pi RP2040 microcontroller. The HAL implements both blocking and async APIs for many peripherals. |  The utilised HAL  |
| [embassy-sync](https://github.com/embassy-rs/embassy/tree/main/embassy-sync) | An Embassy project| Used for mutexes and sending and receiving data over channels |
| [embassy-time](https://github.com/embassy-rs/embassy/tree/main/embassy-time) | An Embassy project | Used for organising the flow of the program via Timer:: |
| [embassy-embedded-hal](https://github.com/embassy-rs/embassy/tree/main/embassy-embedded-hal) | Collection of utilities to use `embedded-hal` and `embedded-storage` traits with Embassy. | Used for the SPI config |
| [embassy-futures](https://github.com/embassy-rs/embassy/tree/main/embassy-futures) | An Embassy project | Used for working with futures during asynchronous development |
| [st7789](https://github.com/almindor/st7789) | Display driver for ST7789 | Used for the display |
| [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics) | 2D graphics library | Used for drawing to the display |

## Links

1. [Idea](https://www.hackster.io/jotrinelectronics/building-a-weather-station-with-raspberry-pi-pico-rp2040-9d5cbb)
2. [Arduino BME280 Library (not Rust, but it was useful as a conceptual insight)](https://github.com/finitespace/BME280)
