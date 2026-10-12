use embassy_time::Delay;
use hd44780_i2c_nostd::LcdI2c;

pub struct Display<I2C> {
    lcd: LcdI2c<I2C>,
}

impl<I2C> Display<I2C>
where
    I2C: embedded_hal_async::i2c::I2c,
{
    pub async fn new(i2c: I2C, addr: u8) -> Self {
        let mut lcd = LcdI2c::new(i2c, addr);
        let mut delay = Delay;
        lcd.init(&mut delay).await.unwrap();
        lcd.set_backlight(true);
        Self { lcd }
    }

    pub async fn write_at(&mut self, row: u8, col: u8, text: &str) {
        let mut delay = Delay;
        let _ = self.lcd.set_cursor(row, col, &mut delay).await;
        let _ = self.lcd.write_str(text, &mut delay).await;
    }
}
