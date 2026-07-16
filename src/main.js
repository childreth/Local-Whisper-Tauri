import './app.css';

const isIndicator = new URLSearchParams(window.location.search).has('indicator');

if (isIndicator) {
  document.body.classList.add('indicator');
} else {
  document.body.classList.add('main-window');
}

async function mount() {
  // Optimization: Use dynamic imports for top-level window components based on the route.
  // This enables Vite code-splitting, ensuring the lightweight indicator window
  // does not load the heavy main App bundle and its dependencies, significantly
  // reducing initialization time and memory footprint.
  const { default: Component } = isIndicator
    ? await import('./lib/IndicatorView.svelte')
    : await import('./App.svelte');

  new Component({
    target: document.getElementById('app'),
  });
}

mount();
