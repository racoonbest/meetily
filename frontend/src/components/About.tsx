import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import Image from 'next/image';
import AnalyticsConsentSwitch from './AnalyticsConsentSwitch';
import { UpdateDialog } from './UpdateDialog';
import { updateService, UpdateInfo } from '@/services/updateService';
import { Button } from './ui/button';
import { Loader2 } from 'lucide-react';
import { toast } from 'sonner';

export function About() {
  const [currentVersion, setCurrentVersion] = useState('0.4.1');
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [isChecking, setIsChecking] = useState(false);
  const [showUpdateDialog, setShowUpdateDialog] = useState(false);

  useEffect(() => {
    getVersion().then(setCurrentVersion).catch(console.error);
  }, []);

  const openProject = async () => {
    try {
      await invoke('open_external_url', { url: 'https://github.com/racoonbest/memo' });
    } catch {
      toast.error('Could not open the project page.');
    }
  };

  const handleCheckForUpdates = async () => {
    setIsChecking(true);
    try {
      const info = await updateService.checkForUpdates(true);
      setUpdateInfo(info);
      if (info.manualUpdates) {
        toast.info('Memo uses manual updates. Check the project page for release information.');
      } else if (info.available) {
        setShowUpdateDialog(true);
      } else {
        toast.success('You are running the latest version');
      }
    } catch {
      toast.error('Could not check for updates.');
    } finally {
      setIsChecking(false);
    }
  };

  return (
    <div className="p-4 space-y-6 max-h-[80vh] overflow-y-auto">
      <div className="text-center space-y-2">
        <Image src="/icon_128x128.png" alt="Memo" width={64} height={64} className="mx-auto" />
        <h2 className="text-xl font-semibold">Memo</h2>
        <p className="text-sm text-muted-foreground">Version {currentVersion}</p>
        <p className="text-sm">Local meeting transcripts, notes, and summaries.</p>
      </div>
      <div className="flex justify-center gap-2">
        <Button variant="outline" size="sm" onClick={openProject}>Project page</Button>
        <Button variant="outline" size="sm" onClick={handleCheckForUpdates} disabled={isChecking}>
          {isChecking && <Loader2 className="mr-2 h-3 w-3 animate-spin" />}
          {isChecking ? 'Checking...' : 'Check for Updates'}
        </Button>
      </div>
      <p className="text-xs text-center text-muted-foreground">
        Based on Meetily Community Edition. Original software copyright Zackriya Solutions, licensed under MIT.
      </p>
      <AnalyticsConsentSwitch />
      <UpdateDialog open={showUpdateDialog} onOpenChange={setShowUpdateDialog} updateInfo={updateInfo} />
    </div>
  );
}
