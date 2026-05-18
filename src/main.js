import App from './App.svelte';
import IndicatorView from './lib/IndicatorView.svelte';

const isIndicator = new URLSearchParams(window.location.search).has('indicator');

if (!isIndicator) {
  // Only the main window uses the global stylesheet; the indicator window
  // must stay transparent so only the pill is visible.
  import('./app.css');
}

const Component = isIndicator ? IndicatorView : App;

const app = new Component({
  target: document.getElementById('app'),
});

export default app;
