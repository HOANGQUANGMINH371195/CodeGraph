import { OrderStore } from './store.mjs';
import { OrderService, NotificationService } from './service.mjs';
import { OutboxRelay } from './relay.mjs';
import { ordersServer, notificationsServer } from './http.mjs';

const mode = process.env.SERVICE ?? 'orders';
if (!['orders', 'notifications'].includes(mode)) throw new Error('invalid service');
const store = new OrderStore(process.env.DB_PATH ?? `${mode}.db`);
const server = mode === 'orders'
  ? ordersServer(new OrderService(store), new OutboxRelay(store, process.env.WORKER_URL ?? 'http://127.0.0.1:8081'))
  : notificationsServer(new NotificationService(store));
server.listen(Number(process.env.PORT ?? (mode === 'orders' ? 8080 : 8081)), process.env.BIND ?? '127.0.0.1');
process.once('SIGTERM', () => server.close(() => store.close()));
