import { createServer } from 'node:http';

async function body(request) {
  let size = 0; const chunks = [];
  for await (const chunk of request) {
    size += chunk.length;
    if (size > 4096) throw new Error('request too large');
    chunks.push(chunk);
  }
  return JSON.parse(Buffer.concat(chunks).toString('utf8'));
}
export function ordersServer(service, relay) {
  return createServer(async (request, response) => {
    try {
      if (request.method === 'POST' && request.url === '/orders') {
        const event = service.create(await body(request));
        response.writeHead(201, {'content-type': 'application/json'});
        response.end(JSON.stringify(event));
      } else if (request.method === 'POST' && request.url === '/dispatch') {
        response.end(JSON.stringify({sent: await relay.dispatch()}));
      } else { response.writeHead(404); response.end(); }
    } catch { response.writeHead(400); response.end('request failed'); }
  });
}
export function notificationsServer(service) {
  return createServer(async (request, response) => {
    try {
      if (request.method !== 'POST' || request.url !== '/events') {
        response.writeHead(404); response.end(); return;
      }
      const inserted = service.consume(await body(request));
      response.end(JSON.stringify({inserted}));
    } catch { response.writeHead(400); response.end('event failed'); }
  });
}
