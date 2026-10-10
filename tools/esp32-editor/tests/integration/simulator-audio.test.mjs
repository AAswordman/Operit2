// Run the shared real Core + rendered Edge fixture through audio verification.
// The full Space regression remains separately available via test:space.
process.env.OPERIT_SIM_AUDIO_ONLY = '1';
await import('./simulator-space.test.mjs');
