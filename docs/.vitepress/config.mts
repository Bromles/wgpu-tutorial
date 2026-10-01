import { defineConfig } from "vitepress";
import { withMermaid } from "vitepress-plugin-mermaid";

const vitePressConfig = defineConfig({
  title: "WGPU Tutorial",
  description: "Руководство по изучению WGPU на Rust для начинающих",
  lang: "ru",
  base: "/wgpu-tutorial/",
  cleanUrls: true,
  markdown: {
    math: true,
  },
  lastUpdated: true,
  vite: {
    build: {
      chunkSizeWarningLimit: 1000,
    },
  },
  head: [["link", { rel: "icon", href: "/wgpu-tutorial/favicon.svg" }]],
  themeConfig: {
    logo: {
      light: "/logo.light.svg",
      dark: "/logo.dark.svg",
    },
    search: {
      provider: "local",
      options: {
        translations: {
          button: {
            buttonText: "Поиск",
          },
          modal: {
            noResultsText: "Нет результатов для",
            footer: {
              navigateText: "для навигации",
              selectText: "выбрать",
              closeText: "закрыть",
            },
          },
        },
      },
    },
    sidebar: [
      { text: "О руководстве", link: "/" },
      {
        text: "Как появляется изображение",
        collapsed: true,
        items: [
          { text: "Изображение и числа", link: "/guide/foundations/image-data/" },
          { text: "CPU, GPU и очередь", link: "/guide/foundations/cpu-gpu-queue/" },
          { text: "Первый кадр", link: "/guide/foundations/first-frame/" },
          { text: "Жизненный цикл поверхности", link: "/guide/foundations/surface-lifecycle/" },
          { text: "От трёх точек к треугольнику", link: "/guide/foundations/first-triangle/" },
          { text: "Граница оконной оболочки", link: "/guide/foundations/shell-triangle/" },
        ],
      },
      {
        text: "Данные и изображение",
        collapsed: true,
        items: [
          { text: "Интерполяция", link: "/guide/foundations/vertex-colors/" },
          { text: "Геометрия в буферах: байты и vertex fetch", link: "/guide/foundations/vertex-fetch/" },
          { text: "Индексированная геометрия", link: "/guide/foundations/indexed-geometry/" },
          { text: "Shader bindings и layout: ресурс и привязка", link: "/guide/foundations/uniform-tint/" },
          { text: "Shader bindings и layout: смешанные поля", link: "/guide/foundations/uniform-params/" },
          { text: "Меняем данные во времени", link: "/guide/foundations/gain-animation/" },
          { text: "Read-only storage и vertex pulling", link: "/guide/foundations/vertex-pulling/" },
          { text: "Immediates и выбор механизма", link: "/guide/foundations/draw-immediates/" },
          { text: "Цвет и его кодирование", link: "/guide/foundations/srgb-mixing/" },
          { text: "Текстуры и sampling: тексели и upload", link: "/guide/foundations/texture-load/" },
          { text: "Текстуры и sampling: UV и фильтр", link: "/guide/foundations/texture-sampling/" },
          { text: "Уменьшение и mipmaps", link: "/guide/foundations/mipmap-minification/" },
        ],
      },
      {
        text: "Пространство и видимость",
        collapsed: true,
        items: [
          { text: "Точки, направления и движение", link: "/guide/space-light/triangle-motion/" },
          { text: "Матрицы и композиция", link: "/guide/space-light/matrix-compose/" },
          { text: "Три измерения и базис", link: "/guide/space-light/basis3d/" },
          { text: "Камера как система координат", link: "/guide/space-light/look-at/" },
          { text: "Проекция и путь на экран", link: "/guide/space-light/ortho-perspective/" },
          { text: "Clip, экран и интерполяция", link: "/guide/space-light/clip-viewport/" },
          { text: "Ориентация и видимость", link: "/guide/space-light/depth-culling/" },
          { text: "Управляемая камера", link: "/guide/space-light/camera-fly/" },
          { text: "Поверхность и UV-швы", link: "/guide/space-light/cube-uv/" },
        ],
      },
      {
        text: "Поверхность и свет",
        collapsed: true,
        items: [
          { text: "Нормали и первый свет", link: "/guide/space-light/lambert/" },
          { text: "Преобразование нормалей", link: "/guide/space-light/normal-matrix/" },
          { text: "Материал и блик", link: "/guide/space-light/blinn-phong/" },
          { text: "Точечный источник и конус", link: "/guide/space-light/light-point/" },
          { text: "Список и сумма источников", link: "/guide/space-light/light-list/" },
          { text: "Over и представление alpha", link: "/guide/space-light/blend-over/" },
          { text: "Порядок и cutout", link: "/guide/space-light/blend-order/" },
          { text: "Геометрия, материал, объект", link: "/guide/space-light/scene-objects/" },
        ],
      },
      {
        text: "Как устроен кадр",
        collapsed: true,
        items: [
          { text: "Базовый compute", link: "/guide/frame/compute-basics/" },
          { text: "Промежуточный кадр: render-to-texture", link: "/guide/frame/render-to-texture/" },
          { text: "Fragment или compute: обработка изображения", link: "/guide/frame/image-pipeline/" },
          { text: "Карта теней", link: "/guide/frame/shadow-mapping/" },
          { text: "Артефакты теней и PCF", link: "/guide/frame/shadow-pcf/" },
          { text: "MSAA и resolve", link: "/guide/frame/msaa-resolve/" },
          { text: "HDR и вывод", link: "/guide/frame/hdr-output/" },
          { text: "Собираем кадр", link: "/guide/frame/full-frame/" },
        ],
      },
      {
        text: "Приложение",
        collapsed: true,
        items: [
          { text: "Почему WebGPU и Rust", link: "/appendix/why-wgpu/" },
          { text: "Что может заблокироваться", link: "/appendix/blocking-and-submissions/" },
        ],
      },
    ],

    socialLinks: [{ icon: "github", link: "https://github.com/Bromles/wgpu-tutorial" }],

    footer: {
      message: "Опубликовано под лицензией CC-BY-4.0",
      copyright: "© Bromles, 2025–2026",
    },

    notFound: {
      title: "Страница не найдена",
      quote: "Дальше живут драконы",
      linkText: "На главную",
    },

    docFooter: {
      prev: "Предыдущая страница",
      next: "Следующая страница",
    },

    lastUpdated: {
      text: "Последнее обновление",
      formatOptions: {
        year: "numeric",
        month: "numeric",
        day: "numeric",
        hour: "numeric",
        minute: "numeric",
        second: "numeric",
        hour12: false,
        forceLocale: true,
      },
    },

    editLink: {
      pattern: "https://github.com/Bromles/wgpu-tutorial/edit/master/docs/:path",
      text: "Редактировать эту страницу",
    },
  },
});

// noinspection JSUnusedGlobalSymbols
export default withMermaid({
  ...vitePressConfig,
});
