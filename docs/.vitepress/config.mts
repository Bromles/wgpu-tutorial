import {defineConfig} from "vitepress";
import {withMermaid} from "vitepress-plugin-mermaid";

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
    head: [["link", {rel: "icon", href: "/wgpu-tutorial/favicon.svg"}]],
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
            {
                text: "Введение",
                items: [{text: "О руководстве", link: "/"}],
            },
            {
                text: "Начало работы",
                collapsed: false,
                items: [
                    {
                        text: "Создание окна",
                        link: "/guide/getting-started/creating-window/",
                    },
                    {
                        text: "Инициализация wgpu",
                        link: "/guide/getting-started/init-wgpu/",
                    },
                    {
                        text: "Первый треугольник",
                        link: "/guide/getting-started/hello-triangle/",
                    },
                ],
            },
            {
                text: "Модель данных GPU",
                collapsed: false,
                items: [
                    {
                        text: "Шейдеры и WGSL",
                        link: "/guide/gpu-data-model/shaders/",
                    },
                    {
                        text: "Вершинные и индексные буферы",
                        link: "/guide/gpu-data-model/buffers/",
                    },
                    {
                        text: "Uniform и bind groups",
                        link: "/guide/gpu-data-model/uniform-bind-groups/",
                    },
                    {
                        text: "Текстуры и сэмплеры",
                        link: "/guide/gpu-data-model/textures/",
                    },
                ],
            },
            {
                text: "Математика для графики",
                collapsed: false,
                items: [
                    {
                        text: "Векторы и матрицы",
                        link: "/guide/math/vectors-matrices/",
                    },
                    {
                        text: "Система координат",
                        link: "/guide/math/coordinate-system/",
                    },
                ],
            },
            {
                text: "3D и камера",
                collapsed: false,
                items: [
                    {
                        text: "Трансформации MVP",
                        link: "/guide/3d/transformations/",
                    },
                    {text: "Depth buffer", link: "/guide/3d/depth-buffer/"},
                    {text: "Камера", link: "/guide/3d/camera/"},
                    {text: "Instancing", link: "/guide/3d/instancing/"},
                ],
            },
            {
                text: "Освещение",
                collapsed: false,
                items: [
                    {
                        text: "Нормали и базовый свет",
                        link: "/guide/lighting/basics/",
                    },
                    {text: "Материалы и свет", link: "/guide/lighting/materials/"},
                    {text: "Тени", link: "/guide/lighting/shadows/"},
                    {
                        text: "Normal Mapping",
                        link: "/guide/lighting/normal-mapping/",
                    },
                ],
            },
            {
                text: "Продвинутый рендер",
                collapsed: false,
                items: [
                    {
                        text: "Render-to-texture",
                        link: "/guide/advanced/render-to-texture/",
                    },
                    {text: "Несколько мешей", link: "/guide/advanced/multiple-meshes/"},
                    {text: "MSAA", link: "/guide/advanced/msaa/"},
                    {
                        text: "HDR и Tone Mapping",
                        link: "/guide/advanced/hdr/",
                    },
                    {
                        text: "Compute Passes",
                        link: "/guide/advanced/compute/",
                    },
                    {
                        text: "Bloom",
                        link: "/guide/advanced/bloom/",
                    },
                    {
                        text: "Particles",
                        link: "/guide/advanced/particles/",
                    },
                ],
            },
            {
                text: "Приложение",
                collapsed: false,
                items: [
                    {text: "Почему WebGPU и Rust", link: "/appendix/why-wgpu/"},
                    {text: "Глоссарий", link: "/appendix/glossary/"},
                ],
            },
        ],

        socialLinks: [
            {icon: "github", link: "https://github.com/Bromles/wgpu-tutorial"},
        ],

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
            pattern:
                "https://github.com/Bromles/wgpu-tutorial/edit/master/docs/:path",
            text: "Редактировать эту страницу",
        },
    },
});

// noinspection JSUnusedGlobalSymbols
export default withMermaid({
    ...vitePressConfig,
});
