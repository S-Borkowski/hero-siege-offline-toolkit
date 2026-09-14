// node --test
import { test } from 'node:test';
import assert from 'node:assert/strict';

import { createProgressRows } from './progress-rows.js';

/** A `schedule` that fires when told, so nothing waits two and a half seconds. */
function manualClock() {
  const pending = [];
  return {
    schedule: (fn) => pending.push(fn),
    fireAll: () => {
      const due = pending.splice(0, pending.length);
      due.forEach((fn) => fn());
    },
    get count() {
      return pending.length;
    },
  };
}

const started = (id) => ({ id, phase: 'started', total: 10 });

test('a finished row goes after its moment', () => {
  const clock = manualClock();
  const rows = createProgressRows({ schedule: clock.schedule });

  rows.receive(started('forgepact'));
  rows.receive({ id: 'forgepact', phase: 'done', version: '1.3.16' });
  assert.equal(rows.current().forgepact.phase, 'done');

  clock.fireAll();
  assert.equal(rows.current().forgepact, undefined, 'the drawer should not keep it');
});

// The reported race, for each phase that schedules a cleanup.
//
// A tool finishes one operation and begins another inside the linger --
// auto-download completing and Update all being clicked, say. The first
// operation's timer fired and deleted the *second* operation's row, so `busy()`
// stopped listing it and the card offered its primary button back while the
// install was still running.
for (const phase of ['done', 'staged', 'downloaded']) {
  test(`a cleanup scheduled by '${phase}' does not remove a newer operation`, () => {
    const clock = manualClock();
    const rows = createProgressRows({ schedule: clock.schedule });

    rows.receive({ id: 'forgepact', phase, version: '1.3.16', reason: 'Hero Siege is running.' });
    assert.equal(clock.count, 1, `'${phase}' should schedule exactly one cleanup`);

    // The next operation starts before that timer fires.
    rows.receive(started('forgepact'));
    assert.equal(rows.current().forgepact.phase, 'started');

    clock.fireAll();

    assert.equal(
      rows.current().forgepact?.phase,
      'started',
      'the running operation must keep its row, or the card stops looking busy',
    );
  });
}

test('the newer operation is still cleaned up when it finishes', () => {
  const clock = manualClock();
  const rows = createProgressRows({ schedule: clock.schedule });

  rows.receive({ id: 'forgepact', phase: 'downloaded' });
  rows.receive(started('forgepact'));
  clock.fireAll(); // the stale timer, which must do nothing

  rows.receive({ id: 'forgepact', phase: 'done', version: '1.3.17' });
  clock.fireAll();
  assert.equal(
    rows.current().forgepact,
    undefined,
    'ownership must not leak: the row still has to go once it really is finished',
  );
});

test('a failure stays up, because it is the only place the reason is written', () => {
  const clock = manualClock();
  const rows = createProgressRows({ schedule: clock.schedule });

  rows.receive({ id: 'forgepact', phase: 'failed', error: 'HTTP 404' });
  assert.equal(clock.count, 0, 'nothing should be scheduled for a failure');
  clock.fireAll();
  assert.equal(rows.current().forgepact.phase, 'failed');
});

test('tools do not clear each other', () => {
  const clock = manualClock();
  const rows = createProgressRows({ schedule: clock.schedule });

  rows.receive({ id: 'forgepact', phase: 'done', version: '1' });
  rows.receive(started('hscraftsim'));
  clock.fireAll();

  assert.equal(rows.current().forgepact, undefined);
  assert.equal(rows.current().hscraftsim.phase, 'started');
});

test('watchers see every change', () => {
  const clock = manualClock();
  const rows = createProgressRows({ schedule: clock.schedule });
  const seen = [];
  rows.watch((next) => seen.push(Object.keys(next).length));

  rows.receive(started('forgepact'));
  rows.receive({ id: 'forgepact', phase: 'done', version: '1' });
  clock.fireAll();

  assert.deepEqual(seen, [1, 1, 0], 'added, updated, removed');
});
