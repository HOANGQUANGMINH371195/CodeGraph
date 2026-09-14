import { DatabaseSync } from 'node:sqlite';
import { readFileSync } from 'node:fs';

const sql = name => readFileSync(new URL(`../sql/${name}.sql`, import.meta.url), 'utf8');
export class OrderStore {
  constructor(path) { this.db = new DatabaseSync(path); this.db.exec(sql('schema')); }
  query(name) { return this.db.prepare(sql(name)); }
  createOrder(id, total) {
    const event = { id: `order:${id}`, type: 'order.created', orderId: id };
    this.db.exec(sql('begin'));
    try {
      this.query('insert-order').run(id, total);
      this.query('enqueue').run(event.id, JSON.stringify(event));
      this.db.exec(sql('commit'));
    } catch (error) { this.db.exec(sql('rollback')); throw error; }
    return event;
  }
  pending() { return this.query('pending').all(); }
  acknowledge(id) { this.query('ack').run(id); }
  consume(event) { return this.query('consume').run(event.id, event.orderId).changes; }
  order(id) { return this.query('order').get(id); }
  notifications() { return this.query('notifications').all(); }
  close() { this.db.close(); }
}
