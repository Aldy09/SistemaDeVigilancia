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
En terminales diferentes:
(utilizamos puerto_servidor = 9090)
- cargo run --bin message_broker_server puerto_servidor 
- cargo run --bin sistema_monitoreo_main ip_servidor puerto_servidor
- cargo run --bin sistema_camaras_main ip_servidor puerto_servidor
- cargo run --bin dron_main id_dron lat_inicial lon_inicial ip_servidor puerto_servidor

## Cómo testear
- cargo test

## Cargo clippy
El comando de clippy que corre el ci es:
- cargo clippy --all-targets --all-features

## Generar la doc
- cargo doc --open