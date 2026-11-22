//! Explicit PC gestures sampled by the existing 200 Hz virtual HID transport.
//! Profiles are input adaptations; all action recognition remains original code.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(u8)]
pub enum Gesture { Right=1,Left=2,Forward=3,Back=4,Up=5,Down=6,Clockwise=7,CounterClockwise=8,Shake=9 }
impl Gesture {
    fn from_code(code:u8)->Option<Self>{Some(match code {1=>Self::Right,2=>Self::Left,3=>Self::Forward,4=>Self::Back,5=>Self::Up,6=>Self::Down,7=>Self::Clockwise,8=>Self::CounterClockwise,9=>Self::Shake,_=>return None})}
}
/// Low nibble is direction; high nibble is a per-device event sequence.
/// Keep the token unchanged after key-up. Zero cancels on focus loss/close.
pub fn token(previous:u8,gesture:Gesture)->u8 {((previous.wrapping_add(16))&0xf0)|gesture as u8}
#[derive(Default)]
pub(crate) struct Pulse {token:u8,step:u16,delay:u16}
impl Pulse {
    pub(crate) fn save(&self,w:&mut gecko::savestate::StateWriter){w.u8(self.token);w.u16(self.step);w.u16(self.delay);}
    pub(crate) fn load(&mut self,r:&mut gecko::savestate::StateReader<'_>)->Result<(),gecko::savestate::StateError>{
        self.token=r.u8()?;self.step=r.u16()?;self.delay=r.u16()?;
        if self.token&15>9{return Err(gecko::savestate::StateError::Corrupt("motion token"));}Ok(())
    }

    pub fn sample(&mut self,token:u8,delay:u16)->Option<[f32;3]>{
        if token&15==0 {self.token=0;self.step=0;return None;}
        let gesture=Gesture::from_code(token&15)?;
        if self.token!=token {self.token=token;self.step=0;self.delay=delay;}
        let step=self.step;self.step=self.step.saturating_add(1);
        if step<self.delay {return Some([0.,0.,1.]);}
        let mut step=step-self.delay;
        if gesture==Gesture::Shake {step%=64;self.step=self.delay+(step+1)%64;}
        else if step>=64 {return None;}
        let mut value=[0.,0.,1.];
        if gesture==Gesture::Shake {
            // Repeated reversals represent a wrist shake, unlike the single
            // directional impulse. Every axis stays within Nunchuk calibration.
            let wave=if (step/8)%2==0 {1.45}else{-1.45};
            value=[wave,wave,1.+wave];
        } else if matches!(gesture,Gesture::Clockwise|Gesture::CounterClockwise) {
            let phase=step as f32/64.*std::f32::consts::TAU;
            let sign=if gesture==Gesture::Clockwise {1.}else{-1.};
            value[0]=2.*phase.sin();value[2]+=sign*2.*phase.cos();
        } else {
            // A distinct impulse followed by mild braking, below the attack
            // impulse's amplitude. No automatic repeat while a key is held.
            let magnitude=match step {0..=7=>2.*step as f32/8.,8..=23=>2.,24..=39=>2.*(40-step) as f32/16.,40..=55=>-0.4,_=>0.};
            let (axis,sign)=match gesture {Gesture::Right=>(0,1.),Gesture::Left=>(0,-1.),Gesture::Forward=>(1,1.),Gesture::Back=>(1,-1.),Gesture::Up=>(2,1.),Gesture::Down=>(2,-1.),_=>unreachable!()};
            value[axis]+=sign*magnitude;
        }
        Some(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinct_axes_sequence_retrigger_and_cancel(){
        for (gesture,axis,sign) in [(Gesture::Right,0,1.),(Gesture::Left,0,-1.),(Gesture::Forward,1,1.),(Gesture::Back,1,-1.),(Gesture::Up,2,1.),(Gesture::Down,2,-1.)] {
            let mut pulse=Pulse::default();let code=token(0,gesture);
            let samples:Vec<_>=(0..70).map(|_|pulse.sample(code,0)).collect();
            let mut expected=[0.,0.,1.];expected[axis]+=sign*2.;assert_eq!(samples[12],Some(expected));
            assert_eq!(samples[69],None);assert_eq!(pulse.sample(code,0),None);
            assert_eq!(pulse.sample(token(code,gesture),0),Some([0.,0.,1.]));
            assert_eq!(pulse.sample(0,0),None);assert_eq!(pulse.sample(code,0),Some([0.,0.,1.]));
        }
    }
    #[test]
    fn held_shake_repeats_and_cancels_without_overflow() {
        let mut pulse=Pulse::default();let code=token(0,Gesture::Shake);
        let values:Vec<_>=(0..64).map(|_|pulse.sample(code,0).unwrap()).collect();
        let reversals=values.windows(2).filter(|pair|pair[0][0].signum()!=pair[1][0].signum()).count();
        assert_eq!(reversals,7);
        for value in &values {assert!((-1.451..=1.451).contains(&value[0]));assert_eq!(value[0],value[1]);assert!((value[2]-value[0]-1.).abs()<0.00001);assert!((-0.451..=2.451).contains(&value[2]));}
        for i in 0..70_000 {assert_eq!(pulse.sample(code,0),Some(values[i%64]));}
        assert_eq!(pulse.sample(0,0),None);
        assert_eq!(pulse.sample(token(code,Gesture::Shake),0),Some(values[0]));
    }
    #[test]
    fn chainsaw_delay_and_opposite_circles(){
        let mut p=Pulse::default();let t=token(0,Gesture::Up);
        for _ in 0..20 {assert_eq!(p.sample(t,20),Some([0.,0.,1.]));}
        for _ in 0..12 {p.sample(t,20);}
        assert_eq!(p.sample(t,20),Some([0.,0.,3.]));
        let mut a=Pulse::default();let mut b=Pulse::default();
        for _ in 0..8 {a.sample(7,0);b.sample(8,0);}
        let a=a.sample(7,0).unwrap();let b=b.sample(8,0).unwrap();
        assert_eq!(a[0],b[0]);assert!((a[2]+b[2]-2.).abs()<0.00001);
    }
}
