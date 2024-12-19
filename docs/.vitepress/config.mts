import {defineConfig} from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
    title: "Learn WGPU",
    description: "Руководство по изучению WGPU на Rust для начинающих",
    base: "/learn-webgpu-rust/",
    //cleanUrls: true,
    markdown: {
        math: true
    },
    themeConfig: {
        // https://vitepress.dev/reference/default-theme-config
        nav: [
            {text: 'Руководство', link: '/getting-started'},
            {text: 'Примеры', link: '/api-examples'}
        ],

        sidebar: [
            {
                text: 'Начало работы',
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
            }
        ],

        socialLinks: [
            {icon: 'github', link: 'https://github.com/Bromles/learn-webgpu-rust'}
        ],

        footer: {
            message: 'Опубликовано под лицензией CC-BY-4.0',
            copyright: '© Bromles, 2024'
        }
    }
})
