import { afterEach } from 'vitest';

// oclif registers process.on('SIGINT', ...) during command.run().
// When vitest tears down the worker with SIGINT, oclif's handler fires and
// throws EEXIT:130 after the test environment has been destroyed. Remove
// those listeners after each test to prevent spurious unhandled errors.
afterEach(() => {
  process.removeAllListeners('SIGINT');
  process.removeAllListeners('SIGTERM');
});
