import './app.css';
import App from './App.svelte';
import IndicatorView from './lib/IndicatorView.svelte';

const isIndicator = new URLSearchParams(window.location.search).has('indicator');

if (isIndicator) {
  document.body.classList.add('indicator');
} else {
  document.body.classList.add('main-window');
}

const Component = isIndicator ? IndicatorView : App;

const app = new Component({
  target: document.getElementById('app'),
});

export default app;
