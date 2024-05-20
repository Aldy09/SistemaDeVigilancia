# Taller de Programación

## Grupo

## Dependencias
Para la interfaz

En ubuntu:
- $ sudo apt-get install libssl-dev
- $ sudo apt-get install libxkbcommon-dev

En mac:
- $ brew install libxkbcommon

## Cómo usar
En dos terminales diferentes, en este orden:
- cargo run --bin message_broker_server
- cargo run --bin message_broker_client

## Cómo testear
- cargo test

## Cargo clippy
El comando de clippy que corre el ci es:
- cargo clippy --all-targets --all-features