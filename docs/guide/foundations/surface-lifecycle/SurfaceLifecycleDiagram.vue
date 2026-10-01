<template>
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1080 1290" role="img" aria-labelledby="title desc" class="guide-diagram">
    <title id="title">Жизненный цикл поверхности в примере surface-lifecycle</title>
    <desc id="desc">Схема состояний, не шкала времени. Resume активирует приложение, suspend удаляет контекст и отменяет retry. Redraw пропускается при неактивности, скрытии или нулевом размере. Будущий срок retry откладывает попытку через WaitUntil. Допустимая попытка создаёт отсутствующий контекст, конфигурирует surface без живого frame и получает изображение. Success показывает кадр. Suboptimal сначала показывает кадр, затем помечает конфигурацию устаревшей без нового redraw. Timeout сохраняет контекст, Outdated помечает конфигурацию, Lost удаляет весь контекст: эти три ветки назначают срок через 100 миллисекунд. Occluded пропускается без таймера и без изменения оконного флага. Validation и другие фатальные ошибки завершают приложение.</desc>
    <defs>
      <marker id="sl-arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto">
        <path d="M0 0 8 4 0 8" fill="none" stroke="var(--vp-c-text-3)"/>
      </marker>
    </defs>
    <g font-family="Arial, sans-serif" font-size="16" fill="#dcdcdc">
      <text x="24" y="34" font-size="24" font-weight="bold" fill="var(--vp-c-text-1)">Поверхность: можно ли получить следующий кадр?</text>
      <text x="24" y="62" fill="var(--vp-c-text-2)">Состояния и переходы; расстояния не означают время исполнения.</text>
      <g stroke="#5a5a74" stroke-width="1.5">
        <rect x="24" y="88" width="498" height="88" rx="8" fill="#24344e"/>
        <rect x="554" y="88" width="502" height="88" rx="8" fill="#2c2c44"/>
        <rect x="24" y="210" width="498" height="92" rx="8" fill="#24344e"/>
        <rect x="650" y="210" width="406" height="92" rx="8" fill="#2c2c44"/>
        <rect x="24" y="340" width="498" height="76" rx="8" fill="#24344e"/>
        <rect x="650" y="340" width="406" height="76" rx="8" fill="#4a3d1f"/>
        <rect x="24" y="458" width="498" height="104" rx="8" fill="#24344e"/>
        <rect x="650" y="458" width="406" height="104" rx="8" fill="#4a2a2a"/>
        <rect x="24" y="600" width="498" height="64" rx="8" fill="#24344e"/>
      </g>
      <text x="40" y="114" font-weight="bold">resumed: active = true</text>
      <text x="40" y="138">Окно только при None; размер читается заново.</text>
      <text x="40" y="161">Сброс occluded/retry; запрос redraw. Повтор: no-op.</text>
      <text x="570" y="114" font-weight="bold">suspended: active = false</text>
      <text x="570" y="138">context = None; retry = None; Wait.</text>
      <text x="570" y="161">Окно остаётся. Повтор безопасен. Возврат: resume.</text>
      <text x="40" y="236" font-weight="bold">Redraw своего WindowId: проверка допуска</text>
      <text x="40" y="261">active и не occluded, width &gt; 0, height &gt; 0?</text>
      <text x="40" y="286">Физический size; нули не заменяются единицами.</text>
      <text x="666" y="236" font-weight="bold">ПРОПУСК: попытка запрещена</text>
      <text x="666" y="261">Retry отменён. Нет configure/acquire.</text>
      <text x="666" y="286">Ждём изменения состояния окна.</text>
      <text x="40" y="367" font-weight="bold">Проверка retry_at</text>
      <text x="40" y="392">Нет срока или срок уже наступил?</text>
      <text x="666" y="367" font-weight="bold">ОЖИДАНИЕ: срок ещё в будущем</text>
      <text x="666" y="392">about_to_wait выбирает WaitUntil.</text>
      <text x="40" y="484" font-weight="bold">ПОДГОТОВКА: retry снят, frame ещё нет</text>
      <text x="40" y="509">context = None? Создать весь Context.</text>
      <text x="40" y="533">Размер изменился или needs_configure?</text>
      <text x="40" y="553">configure; затем needs_configure = false.</text>
      <text x="666" y="484" font-weight="bold">ФАТАЛЬНО: завершение</text>
      <text x="666" y="509">Ошибка setup/лимита или Validation:</text>
      <text x="666" y="533">failure + exit. Uncaptured GPU error:</text>
      <text x="666" y="553">process::exit(1), не retry DeviceLost.</text>
      <text x="40" y="626" font-weight="bold">ACQUIRE: surface.get_current_texture()</text>
      <text x="40" y="650">Семь результатов CurrentSurfaceTexture ниже.</text>
      <g fill="none" stroke="var(--vp-c-text-3)" stroke-width="2" marker-end="url(#sl-arrow)">
        <path d="M273 176 V208"/>
        <path d="M522 254 H648"/>
        <path d="M273 302 V338"/>
        <path d="M522 378 H648"/>
        <path d="M273 416 V456"/>
        <path d="M522 511 H648"/>
        <path d="M273 562 V598"/>
        <path d="M522 632 H853 V564"/>
        <path d="M273 664 V691 H180 V718"/>
        <path d="M273 691 H540 V718"/>
        <path d="M540 691 H900 V718"/>
      </g>
      <text x="558" y="244" fill="var(--vp-c-text-2)">нет</text>
      <text x="286" y="327" fill="var(--vp-c-text-2)">да</text>
      <text x="558" y="367" fill="var(--vp-c-text-2)">нет</text>
      <text x="286" y="442" fill="var(--vp-c-text-2)">да</text>
      <text x="540" y="498" fill="var(--vp-c-text-2)">ошибка</text>
      <text x="610" y="621" fill="var(--vp-c-text-2)">Validation</text>
      <g stroke="#5a5a74" stroke-width="1.5">
        <rect x="24" y="720" width="312" height="194" rx="8" fill="#1f4130"/>
        <rect x="360" y="720" width="360" height="194" rx="8" fill="#4a3d1f"/>
        <rect x="744" y="720" width="312" height="194" rx="8" fill="#2c2c44"/>
        <rect x="24" y="954" width="312" height="142" rx="8" fill="#2c2c44"/>
        <rect x="360" y="954" width="696" height="142" rx="8" fill="#4a3d1f"/>
      </g>
      <text x="40" y="747" font-weight="bold">КАДР: Success / Suboptimal</text>
      <text x="40" y="774">Clear/store; drop прохода.</text>
      <text x="40" y="799">Submit → pre_present_notify</text>
      <text x="40" y="824">→ queue.present(frame).</text>
      <text x="40" y="849">Frame потреблён.</text>
      <text x="40" y="874">Только Suboptimal: затем</text>
      <text x="40" y="899">needs_configure = true.</text>
      <text x="376" y="747" font-weight="bold">RETRY: три разных изменения</text>
      <text x="376" y="777">Timeout: контекст без изменений.</text>
      <text x="376" y="808">Outdated: needs_configure = true.</text>
      <text x="376" y="839">Lost: context = None.</text>
      <text x="376" y="871">Во всех трёх: retry_at =</text>
      <text x="376" y="896">now + 100 мс; frame не получен.</text>
      <text x="760" y="747" font-weight="bold">ПРОПУСК: acquire Occluded</text>
      <text x="760" y="777">Нет frame, нет таймера.</text>
      <text x="760" y="808">Оконный флаг occluded</text>
      <text x="760" y="833">не изменяется.</text>
      <text x="760" y="870">Следующий redraw от ОС</text>
      <text x="760" y="895">снова проверит допуск.</text>
      <g fill="none" stroke="var(--vp-c-text-3)" stroke-width="2" marker-end="url(#sl-arrow)">
        <path d="M180 914 V952"/>
        <path d="M540 914 V952"/>
      </g>
      <text x="40" y="980" font-weight="bold">ОЖИДАНИЕ событий ОС</text>
      <text x="40" y="1006">Нет немедленного redraw</text>
      <text x="40" y="1032">даже после Suboptimal.</text>
      <text x="40" y="1058">Следующий допустимый redraw</text>
      <text x="40" y="1082">вернётся к проверке допуска.</text>
      <text x="376" y="980" font-weight="bold">about_to_wait: Wait / WaitUntil(deadline)</text>
      <text x="376" y="1006">Срок достигнут и окно пригодно? Снять retry, запросить redraw.</text>
      <text x="376" y="1032">Новая попытка возвращается к проверке допуска наверху.</text>
      <text x="376" y="1058">Outdated → configure; Lost → новый Context и configure.</text>
      <text x="376" y="1082">Resize / occlusion / suspend отменяют прежний retry.</text>
      <path d="M24 1122 H1056" stroke="var(--vp-c-divider)"/>
      <text x="24" y="1152" font-weight="bold" fill="var(--vp-c-text-1)">Легенда: цвет обозначает состояние, а не длительность</text>
      <g stroke="#5a5a74">
        <rect x="24" y="1174" width="20" height="20" fill="#24344e"/>
        <rect x="276" y="1174" width="20" height="20" fill="#1f4130"/>
        <rect x="534" y="1174" width="20" height="20" fill="#2c2c44"/>
        <rect x="24" y="1210" width="20" height="20" fill="#4a3d1f"/>
        <rect x="276" y="1210" width="20" height="20" fill="#4a2a2a"/>
      </g>
      <g fill="var(--vp-c-text-2)">
        <text x="54" y="1190">Проверка / подготовка</text>
        <text x="306" y="1190">Кадр передан на показ</text>
        <text x="564" y="1190">Пропуск / ожидание без таймера</text>
        <text x="54" y="1226">Отложенная попытка</text>
        <text x="306" y="1226">Фатальное завершение</text>
      </g>
      <text x="24" y="1263" fill="var(--vp-c-text-3)">100 мс задают deadline попытки, не срок восстановления. Present не подтверждает завершение GPU.</text>
    </g>
  </svg>
</template>
