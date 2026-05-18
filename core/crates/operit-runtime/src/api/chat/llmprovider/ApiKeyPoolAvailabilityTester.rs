pub struct ApiKeyPoolTestState {
    pub tested: i32,
    pub available: i32,
    pub unavailable: i32,
}

pub struct ApiKeyPoolAvailabilityTester;

impl ApiKeyPoolAvailabilityTester {
    pub fn pause(&self) {}

    pub fn is_running(&self) -> bool {
        false
    }

    pub fn start_or_resume(&self) {}
}
