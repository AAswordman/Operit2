// Isolated real Core/Edge workflow using mDNS and the shared fixed C UI.
process.env.OPERIT_SIM_TEST_DEVICE_SPACE_ONLY = '1';
await import('./simulator-space.test.mjs');
