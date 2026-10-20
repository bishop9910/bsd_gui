import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";

const routes: RouteRecordRaw[] = [
  {
    path: "/",
    name: "downloads",
    component: () => import("./views/DownloadsView.vue"),
    meta: { title: "下载任务" },
  },
  {
    path: "/settings",
    name: "settings",
    component: () => import("./views/SettingsView.vue"),
    meta: { title: "设置" },
  },
  {
    path: "/logs",
    name: "logs",
    component: () => import("./views/LogsView.vue"),
    meta: { title: "运行日志" },
  },
  {
    path: "/about",
    name: "about",
    component: () => import("./views/AboutView.vue"),
    meta: { title: "关于" },
  },
  { path: "/:pathMatch(.*)*", redirect: "/" },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

router.afterEach((to) => {
  const title = typeof to.meta.title === "string" ? to.meta.title : "下载任务";
  document.title = `${title} · BSD 下载器`;
});
