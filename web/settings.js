import { POOLS, deepen, isDark, loadFavorites, saveFavorites } from './common.js';

const favorites = loadFavorites();

for (const { color, name, animals } of POOLS) {
  const row = document.createElement('div');
  row.className = 'pool';
  row.style.setProperty('--color', color);
  row.style.setProperty('--deep', deepen(color));
  // null is the random choice.
  const tiles = [null, ...animals].map((animal) => {
    const el = document.createElement('button');
    el.className = isDark(color) ? 'cell dark' : 'cell';
    if (animal) {
      el.classList.add('animal');
      el.style.setProperty('--icon', `url("animals/${animal}.svg")`);
    } else {
      el.textContent = '?';
    }
    el.setAttribute('aria-label', `${name}: ${animal ? animal.replaceAll('-', ' ') : 'random'}`);
    el.addEventListener('click', () => {
      if (animal) favorites[name] = animal;
      else delete favorites[name];
      saveFavorites(favorites);
      mark();
    });
    return { el, animal };
  });
  const mark = () => {
    const current = animals.includes(favorites[name]) ? favorites[name] : null;
    for (const { el, animal } of tiles) el.setAttribute('aria-pressed', String(animal === current));
  };
  mark();
  row.append(...tiles.map((t) => t.el));
  document.getElementById('pools').append(row);
}
