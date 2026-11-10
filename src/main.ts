import { App } from './app';

function bootstrap(): void {
  const root = document.getElementById('root');
  if (root) {
    const app = new App();
    app.mount(root);
  }
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', bootstrap);
} else {
  bootstrap();
}
