#[derive(Debug)]
enum SubscribeReturnCode {
    QoS0 = 0x00,
    QoS1 = 0x01,
    QoS2 = 0x02,
    Failure = 0x80,
}