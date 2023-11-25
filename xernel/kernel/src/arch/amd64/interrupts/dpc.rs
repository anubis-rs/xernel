pub enum DpcState {
    DPCUnbound,
    DPCBound,
    DPCRunning,
}

pub struct Dpc<T> {
    callback: fn(T),
    data: T,
    state: DpcState,
}
