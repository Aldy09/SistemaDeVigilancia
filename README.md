# Taller de Programacion

## Grupo

## Dependencias
Para la interfaz
En ubuntu:
- $ sudo apt-get install libssl-dev
- $ sudo apt-get install libxkbcommon-dev

En mac:
- $ brew install libxkbcommon

## Como usar
En dos terminales diferentes, en este orden:
- cargo run --bin message_broker_server
- cargo run --bin message_broker_client

## Como testear
- cargo test