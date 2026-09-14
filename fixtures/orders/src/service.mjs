export class OrderService {
  constructor(store) { this.store = store; }
  create(input) {
    if (typeof input?.id !== 'string' || !/^[a-z0-9-]{1,64}$/.test(input.id)
      || !Number.isSafeInteger(input.total) || input.total <= 0) throw new Error('invalid order');
    return this.store.createOrder(input.id, input.total);
  }
}

export class NotificationService {
  constructor(store) { this.store = store; }
  consume(event) {
    if (event?.type !== 'order.created' || typeof event.orderId !== 'string'
      || event.id !== `order:${event.orderId}`) throw new Error('invalid event');
    return this.store.consume(event);
  }
}
