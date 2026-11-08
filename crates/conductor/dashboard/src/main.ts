import { createApp } from 'vue';
import App from './App.vue';
import { install } from './api/route';
import './style.css';

// Every read and write goes to the daemon's API (story:instance-api, src/api/route.ts).
install();
createApp(App).mount('#app');
