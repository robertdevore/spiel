interface AppHeaderProps {
  appName: string;
  tagline: string;
}

/**
 * Top-level application header displaying the app name and tagline.
 */
export default function AppHeader({ appName, tagline }: AppHeaderProps) {
  return (
    <header className="app-header">
      <h1 className="app-title">{appName}</h1>
      <p className="app-tagline">{tagline}</p>
    </header>
  );
}
