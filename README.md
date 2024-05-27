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
En tres terminales diferentes, en este orden:
(utilizamos puerto_servidor = 9090)
- cargo run --bin message_broker_server puerto_servidor 
- cargo run --bin message_broker_client
- cargo run --bin sistema_monitoreo ip_client puerto_servidor
- cargo run --bin sistema_camaras ip_client puerto_servidor

## Cómo testear
- cargo test

## Cargo clippy
El comando de clippy que corre el ci es:
- cargo clippy --all-targets --all-features