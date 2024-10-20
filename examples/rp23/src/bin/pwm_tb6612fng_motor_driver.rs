//! # PWM TB6612FNG motor driver
//! 
//! This example shows the use of a TB6612FNG motor driver. The driver is built on top of embedded_hal and the example demonstrates how embassy_rp can be used to interact with ist.

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::block::ImageDef;
use embassy_rp::config::Config;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals;
use embassy_rp::gpio;
use embassy_rp::pwm;
use embassy_time::Duration;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};
use tb6612fng::{DriveCommand, Motor, Tb6612fng};
use assign_resources::assign_resources;

/// Maximum PWM value (fully on)
const PWM_MAX: u16 = 50000;

/// Minimum PWM value (fully off)
const PWM_MIN: u16 = 0;

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

assign_resources! {
    motor: MotorResources {
        standby_pin: PIN_22,
        left_slice: PWM_SLICE6,
        left_pwm_pin: PIN_28,
        left_forward_pin: PIN_21,
        left_backward_pin: PIN_20,
        right_slice: PWM_SLICE5,
        right_pwm_pin: PIN_27,
        right_forward_pin: PIN_19,
        right_backward_pin: PIN_18,
        },
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Config::default());
    let s = split_resources!(p);
    let r = s.motor;

    // we need a standby output and two motors to construct a full TB6612FNG

    // standby pin
    let stby = Output::new(r.standby_pin, gpio::Level::Low);

    // motor A, here defined to be the left motor
    let left_fwd = gpio::Output::new(r.left_forward_pin, gpio::Level::Low);
    let left_bckw = gpio::Output::new(r.left_backward_pin, gpio::Level::Low);
    let mut left_speed = pwm::Config::default();
    left_speed.top = PWM_MAX;
    left_speed.compare_a = PWM_MIN;
    let left_pwm = pwm::Pwm::new_output_a(r.left_slice, r.left_pwm_pin, left_speed);
    let left_motor = Motor::new(left_fwd, left_bckw, left_pwm).unwrap();

    // motor B, here defined to be the right motor
    let right_fwd = gpio::Output::new(r.right_forward_pin, gpio::Level::Low);
    let right_bckw = gpio::Output::new(r.right_backward_pin, gpio::Level::Low);
    let mut right_speed = pwm::Config::default();
    right_speed.top = PWM_MAX;
    right_speed.compare_b = PWM_MIN;
    let right_pwm = pwm::Pwm::new_output_b(r.right_slice, r.right_pwm_pin, right_speed);
    let right_motor = Motor::new(right_fwd, right_bckw, right_pwm).unwrap();

    // construct the motor driver
    let mut control = Tb6612fng::new(left_motor, right_motor, stby).unwrap();

    loop {
        // wake up the motor driver
        info!("end standby");
        control.disable_standby().unwrap();
        Timer::after(Duration::from_millis(100)).await;

        // drive a straight line forward at 20% speed for 5s
        info!("drive straight");
        control.motor_a.drive(DriveCommand::Forward(20)).unwrap();
        control.motor_b.drive(DriveCommand::Forward(20)).unwrap();
        Timer::after(Duration::from_secs(5)).await;

        // coast for 2s
        info!("coast");
        control.motor_a.drive(DriveCommand::Stop).unwrap();
        control.motor_b.drive(DriveCommand::Stop).unwrap();
        Timer::after(Duration::from_secs(2)).await;

        // actively brake
        info!("brake");	
        control.motor_a.drive(DriveCommand::Brake).unwrap();
        control.motor_b.drive(DriveCommand::Brake).unwrap();
        Timer::after(Duration::from_secs(1)).await;

        // slowly turn for 3s
        info!( "turn");
        control.motor_a.drive(DriveCommand::Backward(10)).unwrap();
        control.motor_b.drive(DriveCommand::Forward(10)).unwrap();
        Timer::after(Duration::from_secs(3)).await;

        // and put the driver in standby mode and wait for 5s
        info!( "standby");
        control.enable_standby().unwrap();
        Timer::after(Duration::from_secs(5)).await;
    }
}

