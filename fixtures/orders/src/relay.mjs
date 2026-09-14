export class OutboxRelay {
  constructor(store, endpoint) { this.store = store; this.endpoint = endpoint; }
  async dispatch() {
    let sent = 0;
    for (const event of this.store.pending()) {
      const response = await fetch(`${this.endpoint}/events`, {
        method: 'POST', headers: {'content-type': 'application/json'},
        body: event.payload, signal: AbortSignal.timeout(2000),
      });
      await response.arrayBuffer();
      if (!response.ok) throw new Error('event delivery failed');
      this.store.acknowledge(event.id);
      sent++;
    }
    return sent;
  }
}
