import { Command } from '../protocol/turret-protocol';
import { CommandQueue } from './command-queue';

const aim = (pan: number): Command => ({ op: 'aim', pan, tilt: 0, panSpeed: 0, tiltSpeed: 0 });
const fire: Command = { op: 'fire', mode: 'auto', throttlePct: 50, shots: 1 };

function harness() {
  const sent: Command[] = [];
  const inFlight: (() => void)[] = [];
  const errors: unknown[] = [];
  const queue = new CommandQueue(
    (command) =>
      new Promise<void>((resolve) => {
        sent.push(command);
        inFlight.push(resolve);
      }),
    (error) => errors.push(error),
  );
  const completeWrite = async () => {
    inFlight.shift()?.();
    await Promise.resolve();
    await Promise.resolve();
  };
  return { queue, sent, errors, completeWrite };
}

describe('CommandQueue', () => {
  it('keeps one write in flight and sends the rest in order', async () => {
    const { queue, sent, completeWrite } = harness();

    queue.push({ op: 'arm' });
    queue.push(fire);
    expect(sent).toEqual([{ op: 'arm' }]);

    await completeWrite();
    expect(sent).toEqual([{ op: 'arm' }, fire]);
  });

  it('replaces a queued aim with the newer one', async () => {
    const { queue, sent, completeWrite } = harness();

    queue.push(aim(1));
    queue.push(aim(2));
    queue.push(aim(3));
    await completeWrite();
    await completeWrite();

    expect(sent).toEqual([aim(1), aim(3)]);
  });

  it('replaces a queued pusher command with the newer one', async () => {
    const { queue, sent, completeWrite } = harness();
    const pusher = (pulseUs: number | null): Command => ({ op: 'pusher', pulseUs });

    queue.push(pusher(800));
    queue.push(pusher(810));
    queue.push(pusher(null));
    await completeWrite();
    await completeWrite();

    expect(sent).toEqual([pusher(800), pusher(null)]);
  });

  it('drops everything queued when disarming', async () => {
    const { queue, sent, completeWrite } = harness();

    queue.push(aim(1));
    queue.push({ op: 'arm' });
    queue.push(fire);
    queue.push({ op: 'disarm' });
    await completeWrite();
    await completeWrite();

    expect(sent).toEqual([aim(1), { op: 'disarm' }]);
  });

  it('sends cease fire ahead of queued commands and drops queued fire', async () => {
    const { queue, sent, completeWrite } = harness();

    queue.push(aim(1));
    queue.push(aim(2));
    queue.push(fire);
    queue.push({ op: 'cease-fire' });
    await completeWrite();
    await completeWrite();
    await completeWrite();

    expect(sent).toEqual([aim(1), { op: 'cease-fire' }, aim(2)]);
  });

  it('reports a failed write and carries on', async () => {
    const errors: unknown[] = [];
    const sent: Command[] = [];
    const queue = new CommandQueue(
      async (command) => {
        sent.push(command);
        if (command.op === 'arm') throw new Error('rejected');
      },
      (error) => errors.push(error),
    );

    queue.push({ op: 'arm' });
    queue.push(fire);
    await new Promise((resolve) => setTimeout(resolve));

    expect(errors).toEqual([new Error('rejected')]);
    expect(sent).toEqual([{ op: 'arm' }, fire]);
  });

  it('forgets queued commands on clear', async () => {
    const { queue, sent, completeWrite } = harness();

    queue.push({ op: 'arm' });
    queue.push(fire);
    queue.clear();
    await completeWrite();

    expect(sent).toEqual([{ op: 'arm' }]);
  });
});
