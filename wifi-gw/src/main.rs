#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_25, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_time::{Duration, Timer};
use embedded_io_async::{Read, Write};
use rust_mqtt::client::client::MqttClient;
use rust_mqtt::client::client_config::ClientConfig;

use rust_mqtt::utils::rng_generator::CountingRng;
use static_cell::make_static;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<
        'static,
        Output<'static, PIN_23>,
        PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());

    let fw = include_bytes!("43439A0.bin");
    let clm = include_bytes!("43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs-cli download 43439A0.bin --format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs-cli download 43439A0_clm.bin --format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 224190) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    let state = make_static!(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(wifi_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());
    //let config = embassy_net::Config::Static(embassy_net::Config {
    //    address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
    //    dns_servers: Vec::new(),
    //    gateway: Some(Ipv4Address::new(192, 168, 69, 1)),
    //});

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

    // Init network stack
    let stack = &*make_static!(Stack::new(
        net_device,
        config,
        make_static!(StackResources::<2>::new()),
        seed
    ));

    let mut config = ClientConfig::new(
        rust_mqtt::client::client_config::MqttVersion::MQTTv5,
        CountingRng(20000),
    );
    config.add_max_subscribe_qos(rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1);
    config.add_client_id("client");
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let _buf = [0; 4096];

    config.max_packet_size = 1024;
    let mut recv_buffer = [0; 1024];
    let mut write_buffer = [0; 1024];
    unwrap!(spawner.spawn(net_task(stack)));

    loop {
        //control.join_open(env!("WIFI_NETWORK")).await;
        match control
            .join_wpa2(env!("WIFI_NETWORK"), env!("WIFI_PASSWORD"))
            .await
        {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }
    Timer::after(Duration::from_secs(10)).await;
    // let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(10)));
    let host_addr = embassy_net::Ipv4Address::new(192, 168, 1, 7);
    socket.connect((host_addr, 1883)).await;

    let mut client = MqttClient::<_, 5, _>::new(
        socket,
        &mut write_buffer,
        255,
        &mut recv_buffer,
        255,
        config,
    );
    // And now we can use it!
    client.connect_to_broker().await.unwrap();
    // ha presentation
    client
        .send_message(
            "homeassistant/binary_sensor/gw-status/config",
            br#"{"device_class":"door","payload_on": "ON", "name":"Rs-Gw", "state_topic": "homeassistant/binary_sensor/gw-status/state"}"#,
            rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS0,
            true,
        )
        .await
        .unwrap();
    loop {
        client
            .send_message(
                "homeassistant/binary_sensor/gw-status/state",
                br#"ON"#,
                rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS0,
                true,
            )
            .await
            .unwrap();
        control.gpio_set(0, false).await;
        Timer::after(Duration::from_secs(1)).await;

        // info!("Listening on TCP:1234...");
        // if let Err(e) = socket.accept(1234).await {
        //     warn!("accept error: {:?}", e);
        //     continue;
        // }

        // info!("Received connection from {:?}", socket.remote_endpoint());
        client
            .send_message(
                "homeassistant/binary_sensor/gw-status/state",
                br#"OFF"#,
                rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS0,
                true,
            )
            .await
            .unwrap();
        control.gpio_set(0, true).await;

        Timer::after(Duration::from_secs(1)).await;
        // loop {
        //     let n = match socket.read(&mut buf).await {
        //         Ok(0) => {
        //             warn!("read EOF");
        //             break;
        //         }
        //         Ok(n) => n,
        //         Err(e) => {
        //             warn!("read error: {:?}", e);
        //             break;
        //         }
        //     };

        //     info!("rxd {}", from_utf8(&buf[..n]).unwrap());

        //     match socket.write_all(&buf[..n]).await {
        //         Ok(()) => {}
        //         Err(e) => {
        //             warn!("write error: {:?}", e);
        //             break;
        //         }
        //     };
        // }
    }
}
