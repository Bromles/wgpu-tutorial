import {defineConfig} from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
    title: "Learn WGPU",
    description: "Руководство по изучению WGPU на Rust для начинающих",
    base: "/learn-webgpu-rust/",
    cleanUrls: true,
    markdown: {
        math: true
    },
    lastUpdated: true,
    locales: {
        root: {
            lang: 'ru',
            themeConfig: {
                // https://vitepress.dev/reference/default-theme-config
                nav: [
                    {text: 'Руководство', link: '/getting-started'},
                    {text: 'Примеры', link: '/api-examples'}
                ],

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

                sidebar: [
                    {
                        text: 'Начало работы',
                        collapsed: false,
                        items: [
                            {text: 'Введение', link: '/getting-started'},
                            {text: 'WebGPU и WGPU', link: '/getting-started/webgpu-and-wgpu'},
                            {text: 'Создание окна', link: '/getting-started/creating-window'},
                            {text: 'Первый треугольник', link: '/getting-started/hello-triangle'},
                            {text: 'Шейдеры', link: '/getting-started/shaders'},
                            {text: 'Текстуры', link: '/getting-started/textures'},
                            {text: 'Трансформации', link: '/getting-started/transformations'},
                            {text: 'Система координат', link: '/getting-started/coordinate-system'},
                            {text: 'Камера', link: '/getting-started/camera'},
                        ]
                    },
                    {
                        items: [
                            {text: 'О руководстве', link: '/about'},
                        ]
                    }
                ],

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
