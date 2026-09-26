import type { Locale } from "./locales";

type Messages = readonly [search: string, all: string, findings: string, failed: string, empty: string, scope: string, filter: string, reset: string];

/** Queue-only copy, kept together so every published locale covers this flow. */
export const QUEUE_MESSAGES: Record<Locale, Messages> = {
  en: ["Search names or paths…", "All files", "With findings", "Failed", "No matching files", "Filters only change this view; actions apply to the full queue.", "Filter files", "Clear filters"],
  zh: ["搜索文件名或路径…", "全部文件", "有待清理痕迹", "处理失败", "没有匹配的文件", "筛选仅影响显示，批量操作仍作用于整个队列。", "筛选文件", "清除筛选"],
  "zh-TW": ["搜尋檔名或路徑…", "全部檔案", "有待清理痕跡", "處理失敗", "沒有符合的檔案", "篩選僅影響顯示，批次操作仍套用至整個佇列。", "篩選檔案", "清除篩選"],
  ja: ["名前やパスを検索…", "すべてのファイル", "痕跡あり", "失敗", "一致するファイルなし", "絞り込みは表示のみ変更します。操作はキュー全体に適用されます。", "ファイルを絞り込む", "絞り込みを解除"],
  ko: ["이름 또는 경로 검색…", "모든 파일", "흔적 있음", "실패", "일치하는 파일 없음", "필터는 표시만 변경합니다. 작업은 전체 대기열에 적용됩니다.", "파일 필터", "필터 지우기"],
  de: ["Namen oder Pfade suchen…", "Alle Dateien", "Mit Spuren", "Fehlgeschlagen", "Keine passenden Dateien", "Filter ändern nur die Ansicht. Aktionen gelten für die gesamte Warteschlange.", "Dateien filtern", "Filter zurücksetzen"],
  fr: ["Rechercher un nom ou chemin…", "Tous les fichiers", "Avec des traces", "Échecs", "Aucun fichier correspondant", "Les filtres changent seulement la vue. Les actions concernent toute la file.", "Filtrer les fichiers", "Effacer les filtres"],
  es: ["Buscar nombres o rutas…", "Todos los archivos", "Con rastros", "Fallidos", "No hay archivos coincidentes", "Los filtros solo cambian la vista. Las acciones afectan a toda la cola.", "Filtrar archivos", "Borrar filtros"],
  it: ["Cerca nomi o percorsi…", "Tutti i file", "Con tracce", "Non riusciti", "Nessun file corrispondente", "I filtri modificano solo la vista. Le azioni riguardano l'intera coda.", "Filtra file", "Cancella filtri"],
  pt: ["Pesquisar nomes ou caminhos…", "Todos os ficheiros", "Com vestígios", "Falhas", "Nenhum ficheiro correspondente", "Os filtros alteram apenas a vista. As ações abrangem toda a fila.", "Filtrar ficheiros", "Limpar filtros"],
  "pt-BR": ["Buscar nomes ou caminhos…", "Todos os arquivos", "Com rastros", "Falhas", "Nenhum arquivo correspondente", "Os filtros alteram apenas a visualização. As ações afetam toda a fila.", "Filtrar arquivos", "Limpar filtros"],
  nl: ["Zoek namen of paden…", "Alle bestanden", "Met sporen", "Mislukt", "Geen overeenkomende bestanden", "Filters wijzigen alleen de weergave. Acties gelden voor de hele wachtrij.", "Bestanden filteren", "Filters wissen"],
  da: ["Søg navne eller stier…", "Alle filer", "Med spor", "Mislykkedes", "Ingen matchende filer", "Filtre ændrer kun visningen. Handlinger gælder hele køen.", "Filtrer filer", "Ryd filtre"],
  nb: ["Søk etter navn eller stier…", "Alle filer", "Med spor", "Mislyktes", "Ingen samsvarende filer", "Filtre endrer bare visningen. Handlinger gjelder hele køen.", "Filtrer filer", "Fjern filtre"],
  sv: ["Sök namn eller sökvägar…", "Alla filer", "Med spår", "Misslyckade", "Inga matchande filer", "Filter ändrar bara vyn. Åtgärder gäller hela kön.", "Filtrera filer", "Rensa filter"],
  ca: ["Cerca noms o camins…", "Tots els fitxers", "Amb rastres", "Fallits", "Cap fitxer coincident", "Els filtres només canvien la vista. Les accions afecten tota la cua.", "Filtra fitxers", "Esborra els filtres"],
  cs: ["Hledat názvy nebo cesty…", "Všechny soubory", "Se stopami", "Neúspěšné", "Žádné odpovídající soubory", "Filtry mění pouze zobrazení. Akce platí pro celou frontu.", "Filtrovat soubory", "Zrušit filtry"],
  sk: ["Hľadať názvy alebo cesty…", "Všetky súbory", "So stopami", "Neúspešné", "Žiadne zodpovedajúce súbory", "Filtre menia iba zobrazenie. Akcie platia pre celý rad.", "Filtrovať súbory", "Zrušiť filtre"],
  pl: ["Szukaj nazw lub ścieżek…", "Wszystkie pliki", "Ze śladami", "Nieudane", "Brak pasujących plików", "Filtry zmieniają tylko widok. Działania dotyczą całej kolejki.", "Filtruj pliki", "Wyczyść filtry"],
  ro: ["Caută nume sau căi…", "Toate fișierele", "Cu urme", "Eșuate", "Niciun fișier potrivit", "Filtrele schimbă doar vizualizarea. Acțiunile se aplică întregii cozi.", "Filtrează fișiere", "Șterge filtrele"],
  hr: ["Pretraži nazive ili putanje…", "Sve datoteke", "S tragovima", "Neuspjele", "Nema podudarnih datoteka", "Filtri mijenjaju samo prikaz. Radnje se primjenjuju na cijeli red.", "Filtriraj datoteke", "Ukloni filtre"],
  hu: ["Nevek vagy elérési utak keresése…", "Minden fájl", "Nyomokkal", "Sikertelen", "Nincs megfelelő fájl", "A szűrők csak a nézetet módosítják. A műveletek a teljes sorra vonatkoznak.", "Fájlok szűrése", "Szűrők törlése"],
  el: ["Αναζήτηση ονομάτων ή διαδρομών…", "Όλα τα αρχεία", "Με ίχνη", "Αποτυχημένα", "Δεν βρέθηκαν αρχεία", "Τα φίλτρα αλλάζουν μόνο την προβολή. Οι ενέργειες αφορούν όλη την ουρά.", "Φιλτράρισμα αρχείων", "Εκκαθάριση φίλτρων"],
  tr: ["Ad veya yol ara…", "Tüm dosyalar", "İz içerenler", "Başarısız", "Eşleşen dosya yok", "Filtreler yalnızca görünümü değiştirir. İşlemler tüm kuyruğa uygulanır.", "Dosyaları filtrele", "Filtreleri temizle"],
  ru: ["Поиск по имени или пути…", "Все файлы", "Со следами", "С ошибками", "Совпадений нет", "Фильтры меняют только вид. Действия применяются ко всей очереди.", "Фильтр файлов", "Сбросить фильтры"],
  uk: ["Пошук за назвою чи шляхом…", "Усі файли", "Зі слідами", "З помилками", "Збігів немає", "Фільтри змінюють лише вигляд. Дії застосовуються до всієї черги.", "Фільтр файлів", "Скинути фільтри"],
  ar: ["ابحث عن اسم أو مسار…", "كل الملفات", "تحتوي على آثار", "فشلت", "لا توجد ملفات مطابقة", "تغير المرشحات العرض فقط. تنطبق الإجراءات على قائمة الانتظار كاملة.", "تصفية الملفات", "مسح المرشحات"],
  fa: ["جستجوی نام یا مسیر…", "همه پرونده‌ها", "دارای ردپا", "ناموفق", "پرونده مطابقی نیست", "فیلترها فقط نمایش را تغییر می‌دهند. عملیات روی کل صف انجام می‌شود.", "فیلتر پرونده‌ها", "پاک کردن فیلترها"],
  id: ["Cari nama atau jalur…", "Semua berkas", "Dengan jejak", "Gagal", "Tidak ada berkas yang cocok", "Filter hanya mengubah tampilan. Tindakan berlaku untuk seluruh antrean.", "Filter berkas", "Hapus filter"],
  ms: ["Cari nama atau laluan…", "Semua fail", "Dengan jejak", "Gagal", "Tiada fail sepadan", "Penapis hanya mengubah paparan. Tindakan melibatkan seluruh baris gilir.", "Tapis fail", "Kosongkan penapis"],
  ml: ["പേരോ പാതയോ തിരയുക…", "എല്ലാ ഫയലുകളും", "അവശിഷ്ടങ്ങളുള്ളവ", "പരാജയപ്പെട്ടു", "പൊരുത്തമുള്ള ഫയലുകളില്ല", "ഫിൽട്ടറുകൾ കാഴ്ച മാത്രം മാറ്റുന്നു. പ്രവർത്തനങ്ങൾ മുഴുവൻ ക്യൂവിനും ബാധകമാണ്.", "ഫയലുകൾ ഫിൽട്ടർ ചെയ്യുക", "ഫിൽട്ടറുകൾ മായ്ക്കുക"],
  vi: ["Tìm tên hoặc đường dẫn…", "Tất cả tệp", "Có dấu vết", "Thất bại", "Không có tệp phù hợp", "Bộ lọc chỉ thay đổi hiển thị. Thao tác áp dụng cho toàn bộ hàng đợi.", "Lọc tệp", "Xóa bộ lọc"],
};
