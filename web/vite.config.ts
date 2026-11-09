import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// PC 用 http://localhost（天生安全上下文，麦克风可用）。
// 手机端：Android Chrome 需在 chrome://flags/#unsafely-treat-insecure-origin-as-secure
// 里把 http://<电脑IP>:5173 加入可信列表后重启浏览器，详见 README/测试指引。
// /rpc 代理到本机 Rust SFU，前后端同源，无跨证书问题。
export default defineConfig({
  plugins: [
    vue(),
    {
      // 内嵌/自动化浏览器可能无视 no-cache，强推 no-store 保证 dev 代码新鲜
      name: 'force-no-store',
      configureServer(server) {
        server.middlewares.use((_req, res, next) => {
          res.setHeader('Cache-Control', 'no-store')
          next()
        })
      },
    },
  ],
  server: {
    port: 5173,
    host: true,
    proxy: {
      '/rpc': {
        target: 'https://localhost:7070',
        changeOrigin: true,
        secure: false,
      },
    },
  },
})
