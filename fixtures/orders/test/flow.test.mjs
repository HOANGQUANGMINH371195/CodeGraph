import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { once } from 'node:events';
import { OrderStore } from '../src/store.mjs';
import { OrderService, NotificationService } from '../src/service.mjs';
import { OutboxRelay } from '../src/relay.mjs';
import { ordersServer, notificationsServer } from '../src/http.mjs';

test('POST order commits DB/outbox, delivery crosses HTTP and deduplicates retries', async t => {
  const root = mkdtempSync(join(tmpdir(), 'orders-corpus-'));
  const orders = new OrderStore(join(root, 'orders.db'));
  const notifications = new OrderStore(join(root, 'notifications.db'));
  const servers = [];
  t.after(async () => {
    for (const server of servers) {
      const closed = new Promise(resolve => server.close(resolve));
      server.closeAllConnections(); await closed;
    }
    orders.close(); notifications.close(); rmSync(root, {recursive: true});
  });
  const start = async server => {
    servers.push(server); server.listen(0, '127.0.0.1'); await once(server, 'listening');
    return `http://127.0.0.1:${server.address().port}`;
  };
  const worker = await start(notificationsServer(new NotificationService(notifications)));
  const relay = new OutboxRelay(orders, worker);
  const api = await start(ordersServer(new OrderService(orders), relay));
  const post = async (url, data) => fetch(url, {method: 'POST', body: JSON.stringify(data), signal: AbortSignal.timeout(3000)});
  const created = await post(`${api}/orders`, {id: 'one', total: 100});
  assert.equal(created.status, 201); const event = await created.json();
  assert.equal(orders.order('one').total, 100); assert.equal(orders.pending().length, 1);
  assert.equal(notifications.notifications().length, 0);
  const dispatch = await post(`${api}/dispatch`, {});
  assert.deepEqual(await dispatch.json(), {sent: 1});
  assert.equal(orders.pending().length, 0); assert.equal(notifications.notifications().length, 1);
  const duplicate = await post(`${worker}/events`, event);
  assert.deepEqual(await duplicate.json(), {inserted: 0});
  const failed = await post(`${api}/orders`, {id: 'one', total: 200});
  assert.equal(failed.status, 400); await failed.text();
  assert.equal(orders.order('one').total, 100); assert.equal(orders.pending().length, 0);
  new OrderService(orders).create({id: 'two', total: 50});
  await assert.rejects(new OutboxRelay(orders, `${worker}/wrong`).dispatch());
  assert.equal(orders.pending().length, 1);
  assert.equal(await relay.dispatch(), 1); assert.equal(notifications.notifications().length, 2);
  assert.throws(() => new OrderService(orders).create({id: 'bad', total: -1}));
  assert.equal(orders.order('bad'), undefined);
  orders.query('enqueue').run('order:three', '{}');
  assert.throws(() => new OrderService(orders).create({id: 'three', total: 70}));
  assert.equal(orders.order('three'), undefined); // First insert rolled back after enqueue conflict.
});
