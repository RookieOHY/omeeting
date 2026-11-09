import { createApp } from 'vue'
import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'
import HomePage from './pages/HomePage.vue'
import RoomPage from './pages/RoomPage.vue'
import './style.css'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: HomePage },
    // 房间页：URL 即房间（支持中文，如 /今晚开黑）
    { path: '/:name([^/]+)', component: RoomPage }
  ]
})

createApp(App).use(router).mount('#app')
