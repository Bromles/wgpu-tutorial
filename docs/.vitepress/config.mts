import {defineConfig} from 'vitepress'
import {withMermaid} from "vitepress-plugin-mermaid";

const vitePressConfig = defineConfig({
    title: "WGPU tutorial",
    description: "Руководство по изучению WGPU на Rust для начинающих",
    base: "/wgpu-tutorial/",
    cleanUrls: true,
    markdown: {
        math: true
    },
    lastUpdated: true,
    head: [
        ['link', {rel: 'icon', href: '/wgpu-tutorial/favicon.svg'}],
    ],
    themeConfig: {
        logo: {
            light: '/logo.light.svg',
            dark: '/logo.dark.svg'
        },
        search: {
            provider: 'local',
            options: {
                translations: {
                    button: {
                        buttonText: 'Поиск'
                    },
                    modal: {
                        noResultsText: 'Нет результатов для',
                        footer: {
                            navigateText: 'для навигации',
                            selectText: 'выбрать',
                            closeText: 'закрыть'
                        }
                    }
                }
            }
        },
    },
    locales: {
        root: {
            label: 'Russian',
            lang: 'ru',
            themeConfig: {
                sidebar: {
                    "/": [
                        {
                            items: [
                                {text: 'О руководстве', link: '/'},
                            ]
                        },
                        {
                            text: 'Начало работы',
                            collapsed: false,
                            items: [
                                {text: 'Создание окна', link: '/guide/getting-started/creating-window'},
                                {text: 'Первый треугольник 🚧', link: '/guide/getting-started/hello-triangle'},
                                {text: 'Шейдеры 🚧', link: '/guide/getting-started/shaders'},
                                {text: 'Текстуры 🚧', link: '/guide/getting-started/textures'},
                                {text: 'Трансформации 🚧', link: '/guide/getting-started/transformations'},
                                {text: 'Система координат 🚧', link: '/guide/getting-started/coordinate-system'},
                                {text: 'Камера 🚧', link: '/guide/getting-started/camera'},
                            ]
                        },
                        {
                            text: 'Математика для графики',
                            collapsed: true,
                            items: [
                                {text: 'Введение 🚧', link: '/guide/math'},
                            ]
                        },
                        {
                            text: 'Освещение',
                            collapsed: true,
                            items: [
                                {text: 'Введение 🚧', link: '/guide/lighting'},
                            ]
                        },
                        {
                            text: 'Вычисления на GPU',
                            collapsed: true,
                            items: [
                                {text: 'Введение 🚧', link: '/guide/compute'},
                            ]
                        },
                        {
                            items: [
                                {text: 'Глоссарий 🚧', link: '/guide/glossary'},
                            ]
                        },
                    ],
                    "/examples": [
                        {
                            text: 'Примеры',
                            items: [
                                {text: 'Markdown', link: '/examples/markdown-examples'},
                                {text: 'API', link: '/examples/api-examples'},
                            ]
                        }
                    ]
                },

                socialLinks: [
                    {icon: 'github', link: 'https://github.com/Bromles/wgpu-tutorial'}
                ],

                footer: {
                    message: 'Опубликовано под лицензией CC-BY-4.0',
                    copyright: '© Bromles, 2025'
                },

                notFound: {
                    title: 'Страница не найдена',
                    quote: 'Дальше живут драконы',
                    linkText: 'На главную'
                },

                docFooter: {
                    prev: 'Предыдущая страница',
                    next: 'Следующая страница'
                },

                lastUpdated: {
                    text: 'Последнее обновление',
                    formatOptions: {
                        year: "numeric",
                        month: "numeric",
                        day: "numeric",
                        hour: "numeric",
                        minute: "numeric",
                        second: "numeric",
                        hour12: false,
                        forceLocale: true
                    }
                },

                editLink: {
                    pattern: 'https://github.com/Bromles/wgpu-tutorial/edit/master/docs/:path',
                    text: 'Редактировать эту страницу'
                }
            }
        },
    },
})

// noinspection JSUnusedGlobalSymbols
export default withMermaid({
    ...vitePressConfig,
})
