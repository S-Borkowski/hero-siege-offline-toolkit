// The downloads drawer's rows, and when one of them goes away.
//
// Deliberately free of runes so it can be run under `node --test`: this is the
// third bug to live in frontend bookkeeping that nothing could execute, and a
// module the test runner can import is the difference between a regression test
// and an argument.
//
// `library.svelte.js` owns the reactive copy and subscribes to this.

/**
 * Terminal phases whose row lingers a moment before it goes.
 *
 * `failed` is not here on purpose: a failure stays until the reader does
 * something about it, because it is the only place the reason is written down.
 */
const CLEARS_AFTER_A_MOMENT = ['done', 'staged', 'downloaded'];

/**
 * @param linger    how long a finished row stays up, in ms.
 * @param schedule  injectable `setTimeout`, so a test can fire the timer itself
 *                  rather than waiting two and a half seconds for it.
 */
export function createProgressRows({ linger = 2500, schedule = setTimeout } = {}) {
  let rows = {};
  const watchers = new Set();

  // Tool id -> the event that currently owns that row.
  //
  // Every event takes ownership, and a scheduled cleanup only fires if it still
  // holds it. Without this the timer removed by id alone: a tool that finished
  // one operation and began another within the linger -- auto-download
  // completing and then Update all being clicked, say -- had the *new*
  // operation's row deleted by the *old* one's timer. `busy()` then stopped
  // listing it and the card offered its primary button back while the install
  // was still running.
  const owner = new Map();
  let events = 0;

  const announce = () => watchers.forEach((watch) => watch(rows));

  return {
    /** Called with the new rows whenever they change. */
    watch(fn) {
      watchers.add(fn);
      return () => watchers.delete(fn);
    },

    current() {
      return rows;
    },

    receive(event) {
      if (!event?.id) return;
      const mine = ++events;
      owner.set(event.id, mine);
      rows = { ...rows, [event.id]: event };
      announce();

      if (!CLEARS_AFTER_A_MOMENT.includes(event.phase)) return;
      schedule(() => {
        // Anything at all has happened for this tool since, so the row on
        // screen is not the one this timer was scheduled for.
        if (owner.get(event.id) !== mine) return;
        owner.delete(event.id);
        const { [event.id]: _finished, ...rest } = rows;
        rows = rest;
        announce();
      }, linger);
    },
  };
}
