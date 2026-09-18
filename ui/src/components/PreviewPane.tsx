interface PreviewPaneProps {
  src: string | null;
}

export function PreviewPane({ src }: PreviewPaneProps) {
  return (
    <section className="preview">
      {src ? <img src={src} alt="Prévia do resultado" /> : <p>Gerando prévia…</p>}
    </section>
  );
}
