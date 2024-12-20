import {defineConfig} from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
    title: "WGPU tutorial",
    description: "Руководство по изучению WGPU на Rust для начинающих",
    base: "/wgpu-tutorial/",
    cleanUrls: true,
    markdown: {
        math: true
    },
    lastUpdated: true,
    themeConfig: {
        search: {
            provider: 'local',
            options: {
                locales: ['ru'],
                translations: {
                    button: {
                        buttonText: 'Поиск'
                    },
                    modal: {
                        noResultsText: 'Нет результатов для',
                        footer: {
                            navigateText: 'для навигации',
                            selectText: 'выделить',
                            closeText: 'закрыть'
                        }
                    }
                }
            }
        },
    },
    locales: {
        root: {
            lang: 'ru',
            themeConfig: {
                // https://vitepress.dev/reference/default-theme-config
                nav: [
                    {text: 'Руководство', link: '/guide/getting-started'},
                    {text: 'Примеры', link: '/examples/api-examples'}
                ],

                sidebar: {
                    "/guide": [
                        {
                            text: 'Начало работы',
                            collapsed: false,
                            items: [
                                {text: 'Введение', link: '/guide/getting-started'},
                                {text: 'WebGPU и WGPU', link: '/guide/getting-started/webgpu-and-wgpu'},
                                {text: 'Создание окна', link: '/guide/getting-started/creating-window'},
                                {text: 'Первый треугольник', link: '/guide/getting-started/hello-triangle'},
                                {text: 'Шейдеры', link: '/guide/getting-started/shaders'},
                                {text: 'Текстуры', link: '/guide/getting-started/textures'},
                                {text: 'Трансформации', link: '/guide/getting-started/transformations'},
                                {text: 'Система координат', link: '/guide/getting-started/coordinate-system'},
                                {text: 'Камера', link: '/guide/getting-started/camera'},
                            ]
                        },
                        {
                            items: [
                                {text: 'О руководстве', link: '/guide/about'},
                            ]
                        }
                    ],
                    "examples": [
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
                    {icon: 'github', link: 'https://github.com/Bromles/learn-webgpu-rust'}
                ],

                footer: {
                    message: 'Опубликовано под лицензией CC-BY-4.0',
                    copyright: '© Bromles, 2024'
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
                    pattern: 'https://github.com/Bromles/learn-webgpu-rust/edit/master/docs/:path',
                    text: 'Редактировать эту страницу'
                }
            }
        },
    },
})
