import React from 'react';
const paths:Record<string,string> = {
  grid:'M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z',
  inbox:'M4 4h16v16H4z M4 13h5l2 3h2l2-3h5',
  ticket:'M4 5h16v5a2 2 0 0 0 0 4v5H4v-5a2 2 0 0 0 0-4z M14 5v14',
  link:'M10 13a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-2 2 M14 11a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l2-2',
  folder:'M3 6h7l2 3h9v11H3z', refresh:'M20 7v5h-5 M4 17v-5h5 M6 7a7 7 0 0 1 12-1l2 6 M4 12l2 6a7 7 0 0 0 12-1',
  arrow:'M5 12h14 M13 6l6 6-6 6', plus:'M12 5v14 M5 12h14', search:'M10 18a8 8 0 1 0 0-16 8 8 0 0 0 0 16 M16 16l6 6',
  check:'M5 12l4 4L19 6', clock:'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18 M12 7v5l4 2', close:'M6 6l12 12 M6 18L18 6',
  activity:'M2 12h5l3-8 4 16 3-8h5', chevron:'M9 5l7 7-7 7', settings:'M4 7h16 M4 17h16 M8 4v6 M16 14v6',
  info:'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18 M12 11v6 M12 7v1',
  device:'M7 3h10v18H7z M10 17h4', branch:'M6 8v8 M18 8v1a4 4 0 0 1-4 4h-4 M6 2a3 3 0 1 0 0 6 3 3 0 0 0 0-6 M18 2a3 3 0 1 0 0 0 6 3 3 0 0 0 0-6 M6 16a3 3 0 1 0 0 6 3 3 0 0 0 0-6',
  message:'M4 4h16v13H9l-5 4z M8 8h8 M8 12h5',
};
export function Icon({name,size=18}:{name:string;size?:number}) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]||paths.activity}/></svg>;
}
