# Proyecto Embassy para ESP32-WROOM-32

## Instalación de herramientas

### 1. Toolchain de Espressif para Xtensa

El chip ESP32 usa arquitectura Xtensa, que no tiene soporte en el rustc estandar.
Se necesita el fork de Espressif mediante espup:

    cargo install espup --locked
    espup install

Despues de la instalacion, cargar las variables de entorno:

    . $HOME/export-esp.sh

Para no repetir este paso en cada terminal, agregar al final de ~/.bashrc:

    . $HOME/export-esp.sh

### 2. Herramienta de flasheo y generacion

    cargo install espflash --locked
    cargo install esp-generate --locked

### 3. Acceso al puerto serie de ser el caso

Agregar el usuario al grupo dialout:

    sudo usermod -a -G dialout $USER

Cerrar sesion y volver a entrar para que el cambio tenga efecto.

## Creacion del proyecto

    esp-generate -o esp32 -o esp32-wroom-32 -o embassy -o unstable-hal -o alloc -o log -o esp-backtrace basics

Las opciones indican:
- esp32: chip ESP32 original (arquitectura Xtensa)
- esp32-wroom-32: modulo especifico
- embassy: framework asincrono
- unstable-hal: requerido por embassy en esta version
- alloc: soporte para memoria dinamica
- log y esp-backtrace: sistema de logs y manejo de panics por UART

## Compilar, flashear y monitorear

    cargo run --release

Este comando compila el proyecto, lo graba en la placa y abre el monitor serie.
Para salir del monitor: Ctrl+C

## Comandos utiles

    cargo build --release     Solo compila, sin flashear
    cargo clean               Limpia los artefactos de compilacion
    espflash board-info       Verifica el puerto y el chip detectado

## Verificacion del entorno

    rustup toolchain list

Debe aparecer una linea con el toolchain "esp" ademas del "stable".

    espflash board-info

Debe detectar el chip como "esp32 (revision vX.X)" y mostrar un puerto
serie valido (por ejemplo /dev/ttyUSB0).
