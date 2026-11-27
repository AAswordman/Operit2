// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class AppLocalizationsFr extends AppLocalizations {
  AppLocalizationsFr([String locale = 'fr']) : super(locale);

  @override
  String get terminalProfileSelect => 'Choisir un profil de terminal';

  @override
  String get terminalProfilesUnavailable =>
      'Aucun terminal n\'est disponible sur cet appareil.';

  @override
  String get terminalImplementationNative => 'Terminal natif';

  @override
  String get terminalImplementationProot => 'Terminal Linux PRoot';

  @override
  String get terminalImplementationAndroidSystem => 'Terminal système Android';

  @override
  String get terminalImplementationChroot => 'Chroot (Root)';

  @override
  String get terminalImplementationTermux => 'Termux';

  @override
  String get terminalProfileDirectoryMode => 'Répertoire de démarrage';

  @override
  String get terminalProfileDirectoryHome => 'Répertoire personnel de Termux';

  @override
  String get terminalProfileDirectoryWorkspace =>
      'Espace de travail (stockage partagé)';

  @override
  String get terminalProfileTermuxSetup =>
      'Ouvrez Termux et exécutez pkg install --no-install-recommends python. Ajoutez allow-external-apps = true dans ~/.termux/termux.properties, puis exécutez termux-reload-settings. Dans les paramètres Android, autorisez Operit à exécuter des commandes dans Termux.';

  @override
  String get terminalImplementationQemu => 'Terminal Linux émulé QEMU';

  @override
  String get terminalImplementationArm64 => 'Terminal Linux ARM64 émulé';

  @override
  String get terminalImplementationSystem => 'Terminal système';

  @override
  String get terminalImplementationBrowser => 'VM Linux navigateur';

  @override
  String get terminalInterpreterShell => 'Shell';

  @override
  String terminalBuiltinMacNative(String interpreter) {
    return 'Terminal $interpreter natif macOS';
  }

  @override
  String terminalBuiltinMacSandbox(String interpreter) {
    return 'Terminal $interpreter macOS en bac à sable';
  }

  @override
  String get terminalBuiltinAndroidLinux => 'Terminal Bash Linux PRoot Android';

  @override
  String get terminalBuiltinAndroidSystem => 'Terminal Shell système Android';

  @override
  String get terminalBuiltinLinux => 'Terminal Bash natif Linux';

  @override
  String get terminalBuiltinPosix => 'Terminal Bash natif POSIX';

  @override
  String get terminalBuiltinPrivileged => 'Terminal sh système privilégié';

  @override
  String get terminalBuiltinPowerShellDownload => 'Terminal PowerShell 7';

  @override
  String get terminalBuiltinPowerShell =>
      'Terminal PowerShell 7 intégré à Windows';

  @override
  String get terminalBuiltinWindowsBash => 'Terminal Bash intégré à Windows';

  @override
  String get terminalBuiltinArm64 => 'Terminal Linux ARM64 émulé';

  @override
  String get terminalBuiltinIosSystem => 'Terminal sh système iOS';

  @override
  String get terminalBuiltinOhosLinux =>
      'Terminal Linux émulé QEMU OpenHarmony';

  @override
  String get terminalBuiltinOhosSystem => 'Terminal sh système OpenHarmony';

  @override
  String get terminalBuiltinBrowser => 'Terminal VM Linux navigateur';

  @override
  String get terminalProfilesTitle => 'Profils de terminal';

  @override
  String get terminalProfilesDescription =>
      'Ajoutez des profils de terminal pour cet appareil et choisissez un profil par défaut. Les nouvelles sessions de terminal de l\'espace de travail et de l\'IA partagent ce paramètre ; les sessions existantes conservent leur environnement d\'origine.';

  @override
  String get terminalProfileAdd => 'Ajouter un profil de terminal';

  @override
  String get terminalProfileEdit => 'Modifier le profil de terminal';

  @override
  String get terminalProfileDelete => 'Supprimer le profil de terminal';

  @override
  String get terminalProfileDeleteDescription =>
      'Ce profil ne sera plus disponible pour les nouveaux terminaux. Les sessions en cours restent ouvertes. Sélectionnez un autre profil par défaut avant de supprimer celui-ci.';

  @override
  String get terminalProfileName => 'Nom du profil';

  @override
  String get terminalProfileNameRequired => 'Saisissez un nom de profil';

  @override
  String get terminalProfileImplementation =>
      'Implémentation du terminal et interpréteur';

  @override
  String get terminalProfileRootfs =>
      'Répertoire du système de fichiers racine';

  @override
  String get terminalProfileChrootRootfsHelp =>
      'Nécessite l\'autorisation Root et le chemin absolu vers un système de fichiers racine Linux fiable contenant /bin/bash.';

  @override
  String get terminalProfileRootfsRequired =>
      'Saisissez le chemin du système de fichiers racine.';

  @override
  String get terminalProfileMounts => 'Montages de répertoires';

  @override
  String get terminalProfileAddMount => 'Ajouter un montage';

  @override
  String get terminalProfileMountSource => 'Répertoire hôte';

  @override
  String get terminalProfileMountTarget => 'Répertoire invité';

  @override
  String get terminalProfileMountPathRequired =>
      'Saisissez un chemin de répertoire.';

  @override
  String get terminalProfileRootfsHelp =>
      'Saisissez un chemin absolu accessible vers un système de fichiers racine Linux extrait correspondant à l\'architecture de cet appareil, comme Ubuntu. Android nécessite /bin/bash ; OpenHarmony nécessite /bin/sh. Laissez vide pour utiliser l\'environnement intégré.';

  @override
  String get terminalProfileExecutable =>
      'Chemin de l\'exécutable de l\'interpréteur';

  @override
  String get terminalProfileArguments =>
      'Arguments de lancement (tableau JSON)';

  @override
  String get terminalProfileArgumentsHelp =>
      'Par exemple [\"--login\"]. Les arguments précèdent ceux du shell interactif géré. Indiquez un chemin d\'exécutable lorsque vous spécifiez des options de lancement. PowerShell 5 et 7 peuvent être ajoutés comme profils distincts.';

  @override
  String get terminalProfileArgumentsError =>
      'Saisissez un tableau JSON de chaînes, par exemple [\"--login\"]';

  @override
  String get terminalProfileNoOptions =>
      'Ce terminal est géré par la plateforme et peut être sélectionné par défaut. L\'hôte n\'expose aucune option de lancement supplémentaire.';

  @override
  String get chatPromoteToRoot => 'Promouvoir en conversation principale';

  @override
  String get workspaceUnsavedChangesTitle => 'Modifications non enregistrées';

  @override
  String get workspaceUnsavedChangesMessage =>
      'Enregistrer vos modifications avant de fermer ce fichier ?';

  @override
  String get workspaceDiscardChanges => 'Ignorer les modifications';

  @override
  String get askOperitHint => 'Demander à Operit';

  @override
  String get aiChat => 'Chat IA';

  @override
  String get fullscreenInput => 'Saisie plein écran';

  @override
  String get noticeExpand => 'Développer';

  @override
  String get noticeCollapse => 'Réduire';

  @override
  String get expandInput => 'Agrandir la zone de saisie';

  @override
  String get collapseInput => 'Réduire la zone de saisie';

  @override
  String get settings => 'Paramètres';

  @override
  String get packageManager => 'Gestionnaire de paquets';

  @override
  String get market => 'Marché';

  @override
  String get addAttachment => 'Ajouter une pièce jointe';

  @override
  String get attachmentPhoto => 'Photo';

  @override
  String get attachmentCamera => 'Appareil photo';

  @override
  String get attachmentFile => 'Fichier';

  @override
  String get attachmentScreenContent => 'Contenu de l\'écran';

  @override
  String get attachmentNotifications => 'Notifications actuelles';

  @override
  String get attachmentLocation => 'Position actuelle';

  @override
  String get attachmentPackage => 'Paquet';

  @override
  String get attachmentPackageSelectTitle => 'Sélectionner un paquet';

  @override
  String get attachmentPackageEmpty => 'Aucun paquet disponible';

  @override
  String get attachmentPackageSearchPlaceholder =>
      'Rechercher un nom ou une description de paquet';

  @override
  String get attachmentPackageSearchEmpty => 'Aucun paquet correspondant';

  @override
  String get attachmentPackageKindPackage => 'Paquet';

  @override
  String get attachmentPackageKindSkill => 'Skill';

  @override
  String get attachmentPackageKindMcp => 'MCP';

  @override
  String get attachmentCameraUnavailable =>
      'La capture photo n\'est pas disponible dans le client Flutter';

  @override
  String get clearSearch => 'Effacer la recherche';

  @override
  String chatPendingQueueTitle(int count) {
    return 'Messages en file d\'attente ($count)';
  }

  @override
  String get chatQueueAddMessage => 'Mettre en file d\'attente';

  @override
  String get chatQueueAdded => 'Ajouté à la file d\'attente';

  @override
  String get chatSendOptions => 'Choisir comment envoyer';

  @override
  String get chatQueueAddMessageDescription =>
      'Envoyer après la fin de la réponse en cours (par défaut)';

  @override
  String get chatGuideCurrentTurn => 'Orienter la conversation en cours';

  @override
  String get chatGuideCurrentTurnDescription =>
      'Appliquer lors d\'une requête ultérieure au modèle sans interrompre ce tour';

  @override
  String get chatInterruptAndSend => 'Interrompre et envoyer';

  @override
  String get chatInterruptAndSendDescription =>
      'Arrêter la réponse en cours et envoyer ce message maintenant';

  @override
  String get chatGuidanceAccepted =>
      'Instruction ajoutée pour une requête ultérieure au modèle';

  @override
  String get chatGuidanceTextOnly =>
      'Les instructions ne prennent en charge que le texte. Retirez d\'abord les pièces jointes et la cible de réponse.';

  @override
  String get chatQueueWithdrawToEditor => 'Renvoyer vers l\'éditeur';

  @override
  String get chatPleaseCreateNewChat => 'Veuillez créer une conversation';

  @override
  String get cancel => 'Annuler';

  @override
  String get send => 'Envoyer';

  @override
  String get model => 'Modèle';

  @override
  String get processingInput => 'Traitement de l\'entrée...';

  @override
  String get processingMessage => 'Traitement du message...';

  @override
  String get connectingAiService => 'Connexion au service IA...';

  @override
  String get receivingAiResponse => 'Réception de la réponse de l\'IA...';

  @override
  String get receivingToolResultAiResponse =>
      'Réception de la réponse de l\'IA après l\'exécution de l\'outil...';

  @override
  String get summarizingMemories => 'Synthèse des souvenirs...';

  @override
  String get summaryGenerating => 'Génération du résumé...';

  @override
  String get summaryStageCore => 'Résumé de la tâche principale...';

  @override
  String get summaryStageConstraints =>
      'Résumé des demandes et contraintes de l\'utilisateur...';

  @override
  String get summaryStageFiles => 'Résumé des fichiers clés et du code...';

  @override
  String get summaryStageErrors => 'Résumé des erreurs et correctifs...';

  @override
  String get summaryStageInteraction => 'Résumé des échanges et du contexte...';

  @override
  String get summaryStageNext => 'Résumé des prochaines étapes...';

  @override
  String get summaryStageFormatting => 'Mise en forme du résumé...';

  @override
  String get chatCompressingHistory => 'Compression de l\'historique...';

  @override
  String get executingPlan => 'Exécution du plan...';

  @override
  String executingTool(String toolName) {
    return 'Exécution de l\'outil : $toolName';
  }

  @override
  String processingToolResult(String toolName) {
    return 'Traitement du résultat de l\'outil : $toolName';
  }

  @override
  String get statusWarningAiErrorSummary => 'L\'IA a rencontré une erreur';

  @override
  String get statusWarningAiErrorDetailTitle => 'Cause de l\'erreur de l\'IA';

  @override
  String get toolRunning => 'Outil en cours d\'exécution...';

  @override
  String toolRunningWithName(String toolName) {
    return '$toolName : outil en cours d\'exécution...';
  }

  @override
  String toolStatusWithName(String toolName, String message) {
    return '$toolName : $message';
  }

  @override
  String get close => 'Fermer';

  @override
  String get workspaceCloseOtherTabs => 'Fermer les autres';

  @override
  String get workspaceDockDropTarget => 'Déposer dans l\'espace de travail';

  @override
  String get workspaceOpenWith => 'Ouvrir avec';

  @override
  String get workspaceBuiltInPreview => 'Aperçu intégré';

  @override
  String get workspacePluginUnsavedChanges =>
      'Enregistrez d\'abord dans l\'éditeur ou ignorez les modifications non enregistrées.';

  @override
  String get workspaceCloseTabsToRight => 'Fermer à droite';

  @override
  String get workspaceCloseAllTabs => 'Tout fermer';

  @override
  String get commonExpand => 'Développer';

  @override
  String get commonCollapse => 'Réduire';

  @override
  String get create => 'Créer';

  @override
  String get save => 'Enregistrer';

  @override
  String get delete => 'Supprimer';

  @override
  String get search => 'Rechercher';

  @override
  String get loading => 'Chargement';

  @override
  String get toolApprovalTitle => 'Demande d\'autorisation d\'outil';

  @override
  String get toolApprovalToolLabel => 'Outil';

  @override
  String get toolApprovalActionLabel => 'Action';

  @override
  String get toolApprovalDeny => 'Refuser';

  @override
  String get toolApprovalAllowOnce => 'Autoriser une fois';

  @override
  String get toolApprovalAlwaysAllow => 'Toujours autoriser dans cette session';

  @override
  String get renameConversationTitle => 'Modifier le titre';

  @override
  String get newTitleLabel => 'Nouveau titre';

  @override
  String get deleteConversationTitle => 'Supprimer la conversation ?';

  @override
  String deleteConversationMessage(String title) {
    return 'Supprimer « $title » ?';
  }

  @override
  String get chatHistory => 'Historique des discussions';

  @override
  String get editTitle => 'Modifier le titre';

  @override
  String get moveUp => 'Monter';

  @override
  String get moveDown => 'Descendre';

  @override
  String get pin => 'Épingler';

  @override
  String get unpin => 'Désépingler';

  @override
  String get lock => 'Verrouiller';

  @override
  String get unlock => 'Déverrouiller';

  @override
  String get chatLockedCannotDelete =>
      'Cette discussion est verrouillée et ne peut pas être supprimée';

  @override
  String get messageLocatorTitle => 'Localisateur de messages';

  @override
  String messageLocatorCurrent(int current, int total) {
    return 'Actuel $current / $total';
  }

  @override
  String get messageLocatorSearchHint => 'Rechercher dans les messages';

  @override
  String get messageLocatorInstruction =>
      'Faites défiler la liste ou recherchez pour accéder à un message';

  @override
  String messageLocatorResultCount(int count) {
    return '$count résultats';
  }

  @override
  String get messageLocatorSearchMessages => 'Rechercher des messages';

  @override
  String get messageLocatorViewFavorites => 'Voir les favoris';

  @override
  String get messageLocatorAll => 'Tous';

  @override
  String get messageLocatorFavorites => 'Favoris';

  @override
  String get messageLocatorNoFavorites => 'Aucun message favori pour le moment';

  @override
  String get messageLocatorLoadFailed =>
      'Impossible de charger les messages. Veuillez réessayer.';

  @override
  String messageTransformFailed(String error) {
    return 'Échec du traitement de l\'affichage du message : $error';
  }

  @override
  String get messageLocatorFavoriteFailed =>
      'Impossible de mettre à jour le favori. Veuillez réessayer.';

  @override
  String get messageLocatorAddFavorite => 'Ajouter aux favoris';

  @override
  String get messageLocatorRemoveFavorite => 'Retirer des favoris';

  @override
  String get messageLocatorNoMatches => 'Aucun message correspondant';

  @override
  String get messageSenderUser => 'Utilisateur';

  @override
  String get messageSenderSummary => 'Résumé';

  @override
  String get historyDialogSummary => 'Résumé de l\'historique';

  @override
  String get confirmDeleteSummary => 'Supprimer le résumé';

  @override
  String get confirmDeleteSummaryMessage =>
      'Voulez-vous vraiment supprimer ce résumé ?';

  @override
  String get messageSenderSystem => 'Système';

  @override
  String get messageSenderThinking => 'Réflexion';

  @override
  String get thinkingProcess => 'Processus de réflexion';

  @override
  String thinkingToolsGroupTitleWithCount(int count) {
    return 'Réflexion et appels d\'outils ($count)';
  }

  @override
  String toolsGroupTitleWithCount(int count) {
    return 'Appels d\'outils ($count)';
  }

  @override
  String get searchGroupTitle => 'Recherche';

  @override
  String get searchSources => 'Sources de recherche';

  @override
  String get thinkingSearchGroupTitle => 'Réflexion et recherche';

  @override
  String thinkingSearchToolsGroupTitleWithCount(int count) {
    return 'Réflexion, recherche et appels d\'outils ($count)';
  }

  @override
  String get messageSenderOther => 'Autre';

  @override
  String get hiddenUserMessage => 'Message utilisateur masqué';

  @override
  String get autoTriggeredUserMessage => 'AUTO';

  @override
  String get workspaceSetupTitle => 'Configurer l\'espace de travail';

  @override
  String get workspaceSetupSubtitle =>
      'Fournissez un environnement de fichiers dédié à vos projets IA';

  @override
  String get workspaceCreateTitle => 'Créer un espace de travail';

  @override
  String get workspaceCreateDescription => 'Créer un espace de travail nommé';

  @override
  String get workspaceNameLabel => 'Nom de l\'espace de travail';

  @override
  String get workspaceNameHint => 'Saisissez un nom d\'espace de travail';

  @override
  String get workspaceBindExistingTitle => 'Choisir un espace de travail';

  @override
  String get workspacePickLocalFolder => 'Choisir un dossier sur cet appareil';

  @override
  String get workspaceBindExistingDescription =>
      'Associer un espace de travail existant';

  @override
  String get workspaceAddFolderTitle => 'Ajouter un dossier';

  @override
  String get workspaceAddFolderDescription =>
      'Monter un autre dossier dans l\'espace de travail actuel';

  @override
  String get workspaceProjectTypeDialogTitle => 'Choisir le type de projet';

  @override
  String get workspaceProjectTypeDialogDescription =>
      'Choisissez le type d\'espace de travail par défaut à créer';

  @override
  String get workspaceUnbindTitle => 'Retirer le projet de cette discussion';

  @override
  String get workspaceUnbindDescription =>
      'Détacher cette discussion sans supprimer les fichiers de l\'espace de travail';

  @override
  String get workspaceUnbindConfirmation =>
      'Dissocier l\'espace de travail de cette discussion ? L\'espace de travail et ses fichiers seront conservés, et les autres discussions ne seront pas affectées. Vous pourrez associer un espace de travail à nouveau plus tard.';

  @override
  String get workspaceUnbindFailed =>
      'Échec de la dissociation de l\'espace de travail. Veuillez réessayer.';

  @override
  String get workspaceUnbindChanged =>
      'La discussion ou l\'espace de travail actuel a changé. Annulez et réessayez.';

  @override
  String get workspaceBindDialogTitle =>
      'Choisir un espace de travail existant';

  @override
  String get workspacePathLabel => 'Espace de travail sélectionné';

  @override
  String get workspaceEnvLabel => 'Environnement de l\'espace de travail';

  @override
  String get optionalHint => 'Facultatif';

  @override
  String get workspacePathRequired =>
      'Sélectionnez un dossier d\'espace de travail';

  @override
  String get bind => 'Associer';

  @override
  String get workspaceProjectBlankTitle => 'Espace de travail vide';

  @override
  String get workspaceProjectBlankDescription =>
      'Créer un répertoire d\'espace de travail vide sans fichiers modèles';

  @override
  String get workspaceProjectOfficeTitle => 'Documents bureautiques';

  @override
  String get workspaceProjectOfficeDescription =>
      'Pour l\'édition de documents, le traitement de fichiers et les tâches bureautiques courantes';

  @override
  String get workspaceProjectWebTitle => 'Projet Web';

  @override
  String get workspaceProjectWebDescription =>
      'Pour le développement Web avec HTML/CSS/JavaScript et un serveur local automatique';

  @override
  String get workspaceProjectAndroidTitle => 'Projet Android';

  @override
  String get workspaceProjectAndroidDescription =>
      'Pour le développement Android avec des raccourcis de tâches Gradle courants';

  @override
  String get workspaceProjectFlutterTitle => 'Projet Flutter';

  @override
  String get workspaceProjectFlutterDescription =>
      'Pour le développement multiplateforme Flutter avec un modèle d\'application stable et des commandes courantes';

  @override
  String get workspaceProjectNodeTitle => 'Projet Node.js';

  @override
  String get workspaceProjectNodeDescription =>
      'Pour le développement backend Node.js avec des raccourcis de commandes npm';

  @override
  String get workspaceProjectTypeScriptTitle => 'Projet TypeScript';

  @override
  String get workspaceProjectTypeScriptDescription =>
      'TypeScript + pnpm avec développement typé et surveillance tsc watch';

  @override
  String get workspaceProjectPythonTitle => 'Projet Python';

  @override
  String get workspaceProjectPythonDescription =>
      'Pour le développement Python avec pip et un serveur HTTP';

  @override
  String get workspaceProjectJavaTitle => 'Projet Java';

  @override
  String get workspaceProjectJavaDescription =>
      'Pour le développement Java avec builds Gradle et Maven';

  @override
  String get workspaceProjectGoTitle => 'Projet Go';

  @override
  String get workspaceProjectGoDescription =>
      'Pour le développement Go avec go mod et commandes de compilation';

  @override
  String get version => 'Version';

  @override
  String get author => 'Auteur';

  @override
  String get entry => 'Entrée';

  @override
  String get source => 'Source';

  @override
  String get category => 'Catégorie';

  @override
  String get defaultStatus => 'État par défaut';

  @override
  String get builtIn => 'Intégré';

  @override
  String get external => 'Externe';

  @override
  String get enabledByDefault => 'Activé par défaut';

  @override
  String get disabledByDefault => 'Désactivé par défaut';

  @override
  String get toolPkgResources => 'Ressources ToolPkg';

  @override
  String resourcesCount(int count) {
    return 'Ressources $count';
  }

  @override
  String uiModulesCount(int count) {
    return 'Modules UI $count';
  }

  @override
  String navigationEntriesCount(int count) {
    return 'Entrées de navigation $count';
  }

  @override
  String desktopWidgetsCount(int count) {
    return 'Widgets bureau $count';
  }

  @override
  String workflowTemplatesCount(int count) {
    return 'Modèles de flux de travail $count';
  }

  @override
  String workspaceTemplatesCount(int count) {
    return 'Modèles d\'espace de travail $count';
  }

  @override
  String get pluginConfiguration => 'Configuration du plugin';

  @override
  String get subpackages => 'Sous-paquets';

  @override
  String get toolPkgNoSubpackages => 'Ce ToolPkg ne déclare aucun sous-paquet';

  @override
  String subpackageToolCount(String packageName, int count) {
    return '$packageName · $count outils';
  }

  @override
  String get workflowTemplates => 'Modèles de flux de travail';

  @override
  String get workspaceTemplates => 'Modèles d\'espace de travail';

  @override
  String get disable => 'Désactiver';

  @override
  String get enable => 'Activer';

  @override
  String get environmentVariables => 'Variables d\'environnement';

  @override
  String get packageConfigureEnvironmentVariables =>
      'Configurer les variables d\'environnement';

  @override
  String get packageNoEnvironmentVariables =>
      'Les paquets d\'outils actuellement activés ne déclarent aucune variable d\'environnement.';

  @override
  String get packageEnvironmentOptional => 'Facultatif';

  @override
  String get packageEnvironmentInputRequired =>
      'Valeur de saisie (obligatoire)';

  @override
  String get packageEnvironmentInputOptional => 'Valeur de saisie (facultatif)';

  @override
  String packageEnvironmentDefault(String value) {
    return 'Par défaut : $value';
  }

  @override
  String get required => 'Obligatoire';

  @override
  String get states => 'États';

  @override
  String stateToolSummary(String condition, int toolCount, int excludeCount) {
    return '$condition · $toolCount outils · exclut $excludeCount';
  }

  @override
  String get inherit => 'Hériter';

  @override
  String get tools => 'Outils';

  @override
  String get packageNoTools => 'Ce paquet ne déclare aucun outil';

  @override
  String get permissionsTitle => 'Autorisations';

  @override
  String get clear => 'Effacer';

  @override
  String get noPermissionRecords =>
      'Aucun enregistrement d\'autorisation pour le moment';

  @override
  String get allow => 'Autoriser';

  @override
  String get deny => 'Refuser';

  @override
  String get camera => 'Caméra';

  @override
  String get microphone => 'Microphone';

  @override
  String get protectedMedia => 'Média protégé';

  @override
  String get midiDevice => 'Appareil MIDI';

  @override
  String get browserPermissionRequestTitle => 'Demande d\'autorisation du site';

  @override
  String chatSpeechInputFailed(Object error) {
    return 'Échec de la saisie vocale : $error';
  }

  @override
  String get chatSpeechInputConfigurationRequired =>
      'Sélectionnez une configuration de reconnaissance vocale dans Paramètres > Modèles et IA avant d\'utiliser la saisie vocale.';

  @override
  String get chatSpeechNoTextRecognized => 'Aucune parole reconnue.';

  @override
  String get chatVoiceListening => 'Écoute en cours…';

  @override
  String get chatVoiceStopListening => 'Arrêter la saisie vocale';

  @override
  String get chatVoiceThinking => 'reconnaissance';

  @override
  String get history => 'Historique';

  @override
  String get bookmarks => 'Favoris';

  @override
  String get downloads => 'Téléchargements';

  @override
  String get browserDownloadsAll => 'Tous';

  @override
  String get browserDownloadStorageHint =>
      'Les fichiers sont conservés dans le dossier de téléchargement du navigateur sur l\'appareil du navigateur. Ouvrir copie d\'abord le fichier dans cet espace de travail. Vous pouvez aussi l\'enregistrer directement dans l\'espace de travail.';

  @override
  String get browserDownloadWorkspaceTarget =>
      'Enregistrer dans cet espace de travail :';

  @override
  String get browserDownloadSaveToWorkspace =>
      'Enregistrer dans l\'espace de travail';

  @override
  String get browserDownloadInterrupted => 'Interrompu · retélécharger';

  @override
  String get scripts => 'Scripts';

  @override
  String get zoom => 'Zoom';

  @override
  String get zoomIn => 'Zoom avant';

  @override
  String get zoomOut => 'Zoom arrière';

  @override
  String get desktopMode => 'Version ordinateur';

  @override
  String get clearLocalStorage => 'Effacer le stockage local';

  @override
  String get searchHistory => 'Historique de recherche';

  @override
  String get noDownloadTasks => 'Aucun téléchargement pour le moment';

  @override
  String get openFile => 'Ouvrir le fichier';

  @override
  String get openInOtherApp => 'Ouvrir avec une autre app';

  @override
  String get workspaceFileOpening => 'Ouverture du fichier…';

  @override
  String get workspaceFileOpenTimedOut =>
      'Délai d\'ouverture du fichier dépassé. Veuillez réessayer.';

  @override
  String get openLocation => 'Ouvrir l\'emplacement';

  @override
  String get retry => 'Réessayer';

  @override
  String get removeRecord => 'Supprimer l\'entrée';

  @override
  String get pending => 'En attente';

  @override
  String get completed => 'Terminé';

  @override
  String get failed => 'Échec';

  @override
  String get back => 'Retour';

  @override
  String get forward => 'Suivant';

  @override
  String get stop => 'Arrêter';

  @override
  String get refresh => 'Actualiser';

  @override
  String get home => 'Accueil';

  @override
  String get newTab => 'Nouvel onglet';

  @override
  String get openExternalApplication => 'Ouvrir une application externe';

  @override
  String get browserExternalNavigationFailed =>
      'Impossible d\'ouvrir le lien. Aucune application compatible n\'est installée, le lien est invalide ou le système a rejeté la demande.';

  @override
  String get open => 'Ouvrir';

  @override
  String get ok => 'OK';

  @override
  String get webPage => 'Page web';

  @override
  String get tabs => 'Onglets';

  @override
  String get noBookmarks => 'Aucun favori pour le moment';

  @override
  String get removeBookmark => 'Supprimer le favori';

  @override
  String get addBookmark => 'Ajouter aux favoris';

  @override
  String get menu => 'Menu';

  @override
  String get siteData => 'Données du site';

  @override
  String get clearAllWebViewCookies => 'Effacer tous les cookies WebView';

  @override
  String get clearCookies => 'Effacer les cookies';

  @override
  String get noData => 'Aucune donnée';

  @override
  String get local => 'Local';

  @override
  String get pageLoadFailed => 'Échec du chargement de la page';

  @override
  String get pause => 'Pause';

  @override
  String get resume => 'Reprendre';

  @override
  String get paused => 'En pause';

  @override
  String get cancelled => 'Annulé';

  @override
  String get downloading => 'Téléchargement en cours';

  @override
  String savedTo(String path) {
    return 'Enregistré dans $path';
  }

  @override
  String get sslCertificateError => 'Erreur de certificat SSL';

  @override
  String get edit => 'Modifier';

  @override
  String get files => 'Fichiers';

  @override
  String get terminal => 'Terminal';

  @override
  String get browser => 'Navigateur';

  @override
  String get filePreview => 'Aperçu du fichier';

  @override
  String get workspaceBoundTitle => 'Espace de travail lié';

  @override
  String get selectFile => 'Sélectionner un fichier';

  @override
  String get selectFileDescription =>
      'Sélectionnez un fichier de l\'espace de travail pour le consulter, le modifier ou l\'envoyer à l\'IA';

  @override
  String get openTerminal => 'Ouvrir le terminal';

  @override
  String get openTerminalDescription =>
      'Accéder à la ligne de commande de l\'espace de travail actuel';

  @override
  String get openBrowser => 'Ouvrir le navigateur';

  @override
  String get openBrowserDescription =>
      'Ouvrir une session de navigateur complète, l\'aperçu du projet et l\'automatisation web';

  @override
  String get noWorkspaceBound =>
      'Cette conversation n\'est liée à aucun espace de travail.';

  @override
  String get terminalSessionPlaceholder =>
      'La session de terminal de l\'espace de travail actuel apparaîtra ici.';

  @override
  String get emptyFolder => 'Ce dossier est vide';

  @override
  String get imagePreview => 'Aperçu d\'image';

  @override
  String get audioPreview => 'Aperçu audio';

  @override
  String get videoPreview => 'Aperçu vidéo';

  @override
  String get pdfPreview => 'Aperçu PDF';

  @override
  String get wordPreview => 'Aperçu Word';

  @override
  String get spreadsheetPreview => 'Aperçu de feuille de calcul';

  @override
  String get presentationPreview => 'Aperçu de présentation';

  @override
  String get webPagePreview => 'Aperçu de page web';

  @override
  String get markdownPreview => 'Aperçu Markdown';

  @override
  String get textPreview => 'Aperçu texte';

  @override
  String get file => 'Fichier';

  @override
  String get unsupportedReadOnlyPreview =>
      'Ce type de fichier ne dispose pas d\'un aperçu intégré en lecture seule.';

  @override
  String get cannotPreview => 'Aperçu impossible';

  @override
  String get openProjectInFullBrowser =>
      'Ouvrir le projet dans le navigateur complet';

  @override
  String get openInBrowser => 'Ouvrir dans le navigateur';

  @override
  String get emptySpreadsheet => 'La feuille de calcul est vide';

  @override
  String get settingsCategoryModelTitle => 'Modèles et IA';

  @override
  String get settingsCategoryModelSubtitle => 'Modèles, clés, contexte';

  @override
  String get settingsCategoryModelDescription =>
      'Configurez les connexions aux modèles, choisissez le modèle de chat et gérez la réflexion, le contexte et les capacités multimodales.';

  @override
  String get settingsCategoryLocalModelsTitle => 'Modèles locaux';

  @override
  String get settingsCategoryLocalModelsSubtitle =>
      'LLM, téléchargements, STT / TTS';

  @override
  String get settingsCategoryLocalModelsDescription =>
      'Gérez les modèles locaux et les moteurs d\'inférence installés à la demande.';

  @override
  String get settingsCategoryToolsTitle => 'Outils et autorisations';

  @override
  String get settingsCategoryToolsSubtitle =>
      'Capacités IA, autorisations système, extensions';

  @override
  String get settingsCategoryToolsDescription =>
      'Définissez l\'accès de l\'IA en lecture seule, lecture-écriture ou complet, et consultez l\'état des autorisations système de l\'appareil.';

  @override
  String get settingsCategoryWorkspaceTitle =>
      'Espace de travail et navigateur';

  @override
  String get settingsCategoryWorkspaceSubtitle =>
      'Fichiers, terminal, navigateur';

  @override
  String get settingsCategoryWorkspaceDescription =>
      'Gérez les espaces de travail par défaut, les sessions de terminal, le mode navigateur, les scripts et l\'automatisation web.';

  @override
  String get settingsCategoryGlobalBehaviorTitle => 'Comportement global';

  @override
  String get settingsCategoryGlobalBehaviorSubtitle =>
      'Langue, disposition du chat et saisie';

  @override
  String get settingsCategoryGlobalBehaviorDescription =>
      'Choisissez la langue et configurez la disposition du chat, la saisie et les interactions pour toute l\'application.';

  @override
  String get settingsCategoryAppearanceTitle => 'Apparence et interactions';

  @override
  String get settingsCategoryAppearanceSubtitle =>
      'Thème, bulles et interactions';

  @override
  String get settingsCategoryAppearanceDescription =>
      'Personnalisez le thème global, le média d\'arrière-plan, les bulles de message et les interactions du chat.';

  @override
  String get settingsCategoryDataTitle => 'Données et sauvegarde';

  @override
  String get settingsCategoryDataSubtitle =>
      'Sauvegarde, restauration, stockage';

  @override
  String get settingsCategoryDataDescription =>
      'Sauvegardez et restaurez les chats, les données de plugins et les paramètres de modèles, et gérez le stockage.';

  @override
  String get settingsUsageReportTitle => 'Rapport d\'utilisation des Tokens';

  @override
  String get settingsUsageReportSubtitle =>
      'Utilisation des modèles, cache et coût';

  @override
  String get settingsCategoryAccessLinksTitle => 'Space de l\'appareil';

  @override
  String get settingsCategoryAccessLinksSubtitle =>
      'Connecter, synchroniser, collaborer';

  @override
  String get settingsCategoryAccessLinksDescription =>
      'Gérez ce Space de l\'appareil, connectez des appareils et partagez des données et des routes.';

  @override
  String get settingsCategoryGroupAssistant => 'IA et création';

  @override
  String get settingsCategoryGroupWorkspace =>
      'Espace de travail et automatisation';

  @override
  String get settingsCategoryGroupExperience => 'Affichage et interactions';

  @override
  String get settingsCategoryGroupSystem => 'Données et système';

  @override
  String get settingsGlobalBehaviorChatInputSection => 'Saisie du chat';

  @override
  String get settingsGlobalBehaviorChatLayoutSection => 'Disposition du chat';

  @override
  String get settingsGlobalBehaviorChatScrollNavigatorOnLeft =>
      'Afficher le navigateur de messages à gauche';

  @override
  String get settingsGlobalBehaviorChatScrollNavigatorOnLeftDescription =>
      'Afficher le navigateur en miroir à gauche du chat. Il apparaît à droite par défaut.';

  @override
  String get settingsGlobalBehaviorEnterSendsMessage =>
      'Appuyer sur Entrée pour envoyer';

  @override
  String get settingsGlobalBehaviorEnterSendsMessageDescription =>
      'Si désactivé, Entrée insère un saut de ligne. Ctrl+Entrée et Entrée dans l\'éditeur étendu insèrent toujours un saut de ligne.';

  @override
  String get settingsGlobalBehaviorLongPastedTextAsAttachment =>
      'Convertir un long texte collé en fichier';

  @override
  String get settingsGlobalBehaviorLongPastedTextThreshold =>
      'Seuil de conversion';

  @override
  String settingsGlobalBehaviorLongPastedTextThresholdValue(int count) {
    return '$count caractères';
  }

  @override
  String get settingsComingSoon =>
      'Cette section continuera d\'intégrer les capacités existantes. Les paramètres de modèles et d\'outils sont finalisés en priorité.';

  @override
  String get settingsAdvanced => 'Paramètres avancés';

  @override
  String get settingsActive => 'Actif';

  @override
  String get settingsActivate => 'Activer';

  @override
  String get settingsModelCurrentSection => 'Modèle de chat actuel';

  @override
  String get settingsModelCurrentChatModel => 'Modèle utilisé';

  @override
  String get settingsModelCurrentActive => 'Actif';

  @override
  String get settingsModelMultimodalBadge => 'Multimodal';

  @override
  String get settingsModelNoModels => 'Aucun modèle pour le moment';

  @override
  String settingsModelProviderModelCount(int count) {
    return '$count modèles';
  }

  @override
  String get settingsModelSetCurrentActive => 'Définir comme actif';

  @override
  String get settingsChatStreamOutput => 'Réponse en streaming';

  @override
  String get settingsChatStreamOutputDescription =>
      'Afficher les réponses générées progressivement.';

  @override
  String get settingsModelProfilesSection => 'Profils de modèles';

  @override
  String get settingsModelFunctionFollowChat => 'Suivre le chat';

  @override
  String get settingsModelFunctionFollowChatAll => 'Tout suivre';

  @override
  String get settingsModelFunctionFollowChatHint =>
      'Utilise automatiquement le modèle de chat actuel';

  @override
  String get settingsModelFunctionGroupBackground =>
      'Assistants en arrière-plan';

  @override
  String get settingsModelFunctionGroupMain => 'Modèle de chat principal';

  @override
  String get settingsModelFunctionGroupMultimodal =>
      'Reconnaissance multimodale';

  @override
  String get settingsModelFunctionMappingsSection =>
      'Attribution des modèles par fonction';

  @override
  String get settingsModelFunctionMappingsDescription =>
      'Choisissez le profil de modèle et le modèle précis utilisés pour le chat, les résumés, la mémoire, la reconnaissance d\'images et d\'autres fonctions.';

  @override
  String get settingsModelFunctionMappingsReset => 'Tout réinitialiser';

  @override
  String get settingsModelFunctionMappingsChange => 'Changer';

  @override
  String settingsModelFunctionMappingsSelect(String name) {
    return 'Sélectionner le modèle pour $name';
  }

  @override
  String settingsModelFunctionMappingsCurrent(
    String configName,
    String modelName,
  ) {
    return '$configName · $modelName';
  }

  @override
  String settingsModelFunctionMappingsMissing(
    String providerId,
    String modelId,
  ) {
    return 'Le modèle lié n\'existe pas : $providerId · $modelId';
  }

  @override
  String settingsModelDeleteBlocked(String functions) {
    return 'Ce modèle est utilisé par les fonctions suivantes. Modifiez d\'abord leur attribution : $functions';
  }

  @override
  String settingsModelDeleteProviderBlocked(String functions) {
    return 'Des modèles de ce fournisseur sont utilisés par les fonctions suivantes. Modifiez d\'abord leur attribution : $functions';
  }

  @override
  String settingsModelDeleteProviderConfirm(String name, int count) {
    return 'Supprimer le fournisseur « $name » ? Ses $count modèles seront également supprimés.';
  }

  @override
  String get settingsModelDeleteProviderConfirmAction =>
      'Supprimer le fournisseur';

  @override
  String get settingsTtsDeleteProvider => 'Supprimer le fournisseur TTS';

  @override
  String settingsTtsDeleteProviderConfirm(String name, int count) {
    return 'Supprimer le fournisseur TTS « $name » et ses $count configurations vocales ?';
  }

  @override
  String settingsTtsDeleteProviderFailed(String error) {
    return 'Échec de la suppression du fournisseur TTS : $error';
  }

  @override
  String get settingsTtsCurrentConfigCannotDelete =>
      'La configuration TTS actuellement utilisée ne peut pas être supprimée.';

  @override
  String get settingsModelChatAutoGlmWarning =>
      'AutoGLM ne peut pas être utilisé comme modèle de chat principal. Le chat et le contrôle de l\'interface utilisent des attributions distinctes ; choisissez un autre grand modèle.';

  @override
  String get settingsModelFunctionChat => 'Chat';

  @override
  String get settingsModelFunctionChatDescription =>
      'Modèle utilisé pour les réponses principales de la conversation.';

  @override
  String get settingsModelFunctionSummary => 'Résumé';

  @override
  String get settingsModelFunctionSummaryDescription =>
      'Modèle utilisé pour les résumés automatiques de longs contextes.';

  @override
  String get settingsModelFunctionTitleGeneration => 'Titre de conversation';

  @override
  String get settingsModelFunctionTitleGenerationDescription =>
      'Modèle utilisé pour résumer le premier message et les pièces jointes en titre de conversation.';

  @override
  String get settingsModelFunctionMemory => 'Mémoire';

  @override
  String get settingsModelFunctionMemoryDescription =>
      'Modèle utilisé pour extraire, organiser et mettre à jour les souvenirs.';

  @override
  String get settingsModelFunctionUiController => 'Contrôle de l\'interface';

  @override
  String get settingsModelFunctionUiControllerDescription =>
      'Modèle utilisé pour le contrôle d\'interface et la planification d\'actions légères.';

  @override
  String get settingsModelFunctionTranslation => 'Traduction';

  @override
  String get settingsModelFunctionTranslationDescription =>
      'Modèle utilisé pour traduire du texte et du contenu localisé.';

  @override
  String get settingsModelFunctionGrep => 'Recherche de texte';

  @override
  String get settingsModelFunctionGrepDescription =>
      'Modèle utilisé pour filtrer les résultats de recherche et évaluer les correspondances textuelles.';

  @override
  String get settingsModelFunctionRoleResponsePlanner =>
      'Planificateur de réponses en groupe';

  @override
  String get settingsModelFunctionRoleResponsePlannerDescription =>
      'Modèle utilisé pour planifier les rôles et l\'ordre de prise de parole dans les conversations de groupe.';

  @override
  String get settingsModelFunctionImageRecognition =>
      'Reconnaissance d\'images';

  @override
  String get settingsModelFunctionImageRecognitionDescription =>
      'Modèle utilisé pour comprendre les images et en extraire le contenu.';

  @override
  String get settingsModelFunctionAudioRecognition => 'Reconnaissance audio';

  @override
  String get settingsModelFunctionAudioRecognitionDescription =>
      'Modèle utilisé pour comprendre l\'audio et en extraire le contenu.';

  @override
  String get settingsModelFunctionVideoRecognition => 'Reconnaissance vidéo';

  @override
  String get settingsModelFunctionVideoRecognitionDescription =>
      'Modèle utilisé pour comprendre la vidéo et en extraire le contenu.';

  @override
  String get settingsModelFunctionImageUnsupported =>
      'La saisie directe d\'images est désactivée pour le profil de modèle sélectionné.';

  @override
  String get settingsModelFunctionAudioUnsupported =>
      'La saisie directe d\'audio est désactivée pour le profil de modèle sélectionné.';

  @override
  String get settingsModelFunctionVideoUnsupported =>
      'La saisie directe de vidéo est désactivée pour le profil de modèle sélectionné.';

  @override
  String get settingsModelCreateProfile => 'Nouveau profil de modèle';

  @override
  String get settingsModelEditProfile => 'Modifier le profil de modèle';

  @override
  String get settingsModelProfileName => 'Nom du profil';

  @override
  String get settingsModelApiEndpoint => 'Point de terminaison API';

  @override
  String get settingsModelModelNames => 'Noms des modèles';

  @override
  String get settingsModelApiKey => 'Clé API';

  @override
  String get settingsModelShowApiKey => 'Afficher la clé API';

  @override
  String get settingsModelHideApiKey => 'Masquer la clé API';

  @override
  String get settingsModelCodexLogin => 'Connexion ChatGPT';

  @override
  String get settingsModelCodexLoginTitle => 'Se connecter à ChatGPT Codex';

  @override
  String get settingsModelCodexLoginDescription =>
      'OpenAI Codex utilise une connexion par abonnement ChatGPT au lieu d\'une clé API.';

  @override
  String settingsModelCodexSignedIn(String account) {
    return 'Connecté en tant que $account';
  }

  @override
  String get settingsModelCodexSignedOut => 'Non connecté à ChatGPT';

  @override
  String get settingsModelCodexBrowser => 'Connexion via navigateur';

  @override
  String get settingsModelCodexDevice => 'Code d\'appareil';

  @override
  String get settingsModelCodexWaiting => 'En attente d\'autorisation ChatGPT…';

  @override
  String get settingsModelCodexDeviceInstructions =>
      'Saisissez ce code sur la page ouverte, puis revenez dans Operit.';

  @override
  String get settingsModelCodexOpenPage => 'Ouvrir la page de vérification';

  @override
  String get settingsModelCodexLogout => 'Se déconnecter';

  @override
  String settingsModelCodexLoginFailed(String error) {
    return 'Échec de la connexion à ChatGPT : $error';
  }

  @override
  String get settingsModelApiKeyPool => 'Pool de clés API';

  @override
  String get settingsModelApiKeyPoolDescription =>
      'Continue d\'utiliser une clé valide. Bascule en cas d\'erreur d\'authentification, de quota, de limite de débit ou d\'échec temporaire ; les clés en pause sont ignorées.';

  @override
  String settingsModelApiKeyPoolCount(int count) {
    return '$count clés';
  }

  @override
  String get settingsModelApiKeyPoolEmpty =>
      'Aucune clé pour le moment. Ajoutez une clé avant d\'activer ce pool.';

  @override
  String get settingsModelAddApiKey => 'Ajouter une clé';

  @override
  String get settingsModelEditApiKey => 'Modifier la clé';

  @override
  String get settingsModelApiKeyName => 'Nom de la clé';

  @override
  String get settingsModelApiKeyEnabled => 'Activer cette clé';

  @override
  String get settingsModelPoolAddSingle => 'Ajouter la clé unique actuelle';

  @override
  String get settingsModelPoolPaste =>
      'Collez une clé par ligne pour en ajouter plusieurs à la fois.';

  @override
  String get settingsModelPoolNoEligible =>
      'Activez au moins une clé non vide qui n\'est pas marquée comme indisponible.';

  @override
  String get settingsModelPoolUntested => 'Non testée';

  @override
  String get settingsModelPoolAvailable => 'Disponible';

  @override
  String get settingsModelPoolUnavailable => 'Indisponible';

  @override
  String get settingsModelPoolClearMarks => 'Réinitialiser l\'état des clés';

  @override
  String get settingsModelPoolTest => 'Tester les clés non testées';

  @override
  String get settingsModelPoolNeedsModel =>
      'Enregistrez le fournisseur et ajoutez un modèle avant de tester les clés.';

  @override
  String get settingsModelProviderId => 'ID du fournisseur';

  @override
  String get settingsModelProvidersSection => 'Fournisseurs LLM';

  @override
  String get settingsModelProviderType => 'Type de fournisseur';

  @override
  String settingsModelProviderTypeOption(String name, String original) {
    return '$name ($original)';
  }

  @override
  String get settingsModelProviderTypeOpenai => 'OpenAI';

  @override
  String get settingsModelProviderTypeXai => 'xAI';

  @override
  String get settingsModelProviderTypeOpenaiResponses => 'OpenAI Responses';

  @override
  String get settingsModelProviderTypeOpenaiCodex => 'OpenAI Codex';

  @override
  String get settingsModelProviderTypeOpenaiResponsesGeneric =>
      'Compatible OpenAI Responses';

  @override
  String get settingsModelProviderTypeOpenaiGeneric => 'Compatible OpenAI';

  @override
  String get settingsModelProviderTypeAnthropic => 'Anthropic';

  @override
  String get settingsModelProviderTypeAnthropicGeneric =>
      'Compatible Anthropic';

  @override
  String get settingsModelProviderTypeGoogle => 'Google Gemini';

  @override
  String get settingsModelProviderTypeGeminiGeneric => 'Compatible Gemini';

  @override
  String get settingsModelProviderTypeBaidu => 'Baidu';

  @override
  String get settingsModelProviderTypeAliyun => 'Aliyun';

  @override
  String get settingsModelProviderTypeXunfei => 'Xunfei';

  @override
  String get settingsModelProviderTypeZhipu => 'Zhipu AI';

  @override
  String get settingsModelProviderTypeBaichuan => 'Baichuan';

  @override
  String get settingsModelProviderTypeMoonshot => 'Moonshot';

  @override
  String get settingsModelProviderTypeMimo => 'MiMo';

  @override
  String get settingsModelProviderTypeDeepseek => 'DeepSeek';

  @override
  String get settingsModelProviderTypeMistral => 'Mistral';

  @override
  String get settingsModelProviderTypeSiliconflow => 'SiliconFlow';

  @override
  String get settingsModelProviderTypeOpenrouter => 'OpenRouter';

  @override
  String get settingsModelProviderTypeOpencode => 'OpenCode';

  @override
  String get settingsModelProviderTypeFourRouter => '4Router';

  @override
  String get settingsModelProviderTypeNousPortal => 'Nous Portal';

  @override
  String get settingsModelProviderTypeInfiniai => 'InfiniAI';

  @override
  String get settingsModelProviderTypeAlipayBailing => 'Alipay Bailing';

  @override
  String get settingsModelProviderTypeDoubao => 'Doubao';

  @override
  String get settingsModelProviderTypeNvidia => 'NVIDIA';

  @override
  String get settingsModelProviderTypeLmstudio => 'LM Studio';

  @override
  String get settingsModelProviderTypeOllama => 'Ollama';

  @override
  String get settingsModelProviderTypeOpenaiLocal => 'OpenAI Local';

  @override
  String get settingsModelProviderTypeLocalModel => 'Modèle local';

  @override
  String get settingsModelProviderTypeLocalGateway => 'Passerelle locale';

  @override
  String get settingsModelLocalGatewayHint =>
      'Créez ce fournisseur sur l’appareil exécutant Magpie, CC Switch ou une autre passerelle. Les adresses locales désignent cet appareil ; les autres appareils y accèdent par route. Compatible avec /v1/chat/completions, /v1/responses et /v1/messages. Gardez cet appareil en ligne et autorisez l’exécution à distance.';

  @override
  String localModelsLoadFailed(Object error) {
    return 'Échec du chargement de l\'état des modèles locaux : $error';
  }

  @override
  String localModelsOperationFailed(Object error) {
    return 'Échec de l\'opération sur le modèle local : $error';
  }

  @override
  String get localModelsAddModel => 'Parcourir en ligne';

  @override
  String get localModelsImportFromDevice => 'Importer depuis l\'appareil';

  @override
  String get localModelsImportGgufHint =>
      'Choisissez un modèle de chat GGUF complet pour llama.cpp.';

  @override
  String get localModelsImportMnnHint =>
      'Sélectionnez ensemble les fichiers extraits du modèle MNN, y compris config.json et les poids.';

  @override
  String get localModelsImportInvalidGguf =>
      'Sélectionnez un seul fichier .gguf.';

  @override
  String get localModelsImportMissingConfig =>
      'Le fichier config.json est manquant dans les fichiers sélectionnés.';

  @override
  String get localModelsImportFileSizeRequired =>
      'Chaque fichier sélectionné doit être non vide et indiquer sa taille.';

  @override
  String get localModelsImportChangeFiles => 'Choisir à nouveau';

  @override
  String localModelsImportFileCount(int count) {
    return '$count fichiers sélectionnés';
  }

  @override
  String get localModelsImportName => 'Nom du modèle';

  @override
  String get localModelsImportCopyNotice =>
      'Les fichiers sont copiés dans le stockage de l\'application et restent sur cet appareil.';

  @override
  String get localModelsImportCopying => 'Copie des fichiers du modèle…';

  @override
  String get localModelsImportFinishing =>
      'Vérification et enregistrement du modèle…';

  @override
  String get localModelsImportComplete =>
      'Modèle importé. Ouvrez Exécuter et configurer pour l\'essayer.';

  @override
  String get localModelsImportedFile => 'Import depuis l\'appareil';

  @override
  String get localModelsFilterDownloads => 'Téléchargements';

  @override
  String get localModelsAllTypes => 'Tous les types';

  @override
  String get localModelsNoMatches => 'Aucun modèle correspondant';

  @override
  String get localModelsResetFilters => 'Réinitialiser les filtres';

  @override
  String get localModelsDetails => 'Détails du modèle';

  @override
  String get localModelsMoreActions => 'Plus d\'actions';

  @override
  String localModelsResultCount(int count, int total) {
    return 'Affichage de $count sur $total modèles';
  }

  @override
  String get localModelsHubRepositoryTab => 'Dépôt';

  @override
  String get localModelsDownloadFailed => 'Échec du téléchargement';

  @override
  String get localModelsDownloadPausedLabel => 'En pause';

  @override
  String get localModelsCatalog => 'Catalogue de modèles';

  @override
  String get localModelsCategorySpeechToText =>
      'Modèles de reconnaissance vocale';

  @override
  String get localModelsCategoryTextToSpeech => 'Modèles de synthèse vocale';

  @override
  String get localModelsCategoryChat => 'Modèles LLM';

  @override
  String get localModelsCategoryEmbedding => 'Modèles d\'embedding';

  @override
  String get localModelsInstalledEngines => 'Moteurs installés';

  @override
  String get localModelsNoInstalledEngines =>
      'Aucun moteur d\'inférence local n\'est installé sur cette plateforme.';

  @override
  String get localModelsDeleteModelTitle => 'Supprimer le modèle local';

  @override
  String localModelsDeleteModelMessage(Object modelName) {
    return 'Supprimer les fichiers du modèle $modelName ?';
  }

  @override
  String get localModelsDeleteEngineTitle => 'Supprimer le moteur local';

  @override
  String localModelsDeleteEngineMessage(Object engineName, Object version) {
    return 'Supprimer $engineName $version ?';
  }

  @override
  String get localModelsCancelling => 'Mise en pause';

  @override
  String localModelsDownloadPaused(Object downloaded, Object total) {
    return 'En pause : $downloaded / $total';
  }

  @override
  String get localModelsDownloadInstalling =>
      'Téléchargement terminé, installation en cours';

  @override
  String localModelsDownloading(Object downloaded, Object total) {
    return 'Téléchargement : $downloaded / $total';
  }

  @override
  String localModelsLicense(Object license) {
    return 'Licence : $license';
  }

  @override
  String get localModelsPlatformCompatible => 'Compatible avec la plateforme';

  @override
  String get localModelsPlatformIncompatible =>
      'Incompatible avec la plateforme';

  @override
  String get localModelsModelInstalled => 'Modèle installé';

  @override
  String get localModelsModelNotInstalled => 'Modèle non installé';

  @override
  String get localModelsEngineInstalled => 'Moteur installé';

  @override
  String get localModelsEngineNotInstalled => 'Moteur non installé';

  @override
  String get localModelsVerifyModelAndEngine =>
      'Vérifier le modèle et le moteur';

  @override
  String get localModelsDeleteModel => 'Supprimer le modèle';

  @override
  String get localModelsPauseDownload => 'Mettre en pause';

  @override
  String get localModelsDeleteDownload => 'Supprimer le téléchargement';

  @override
  String get localModelsResumeDownload => 'Reprendre';

  @override
  String get localModelsInstalling => 'Installation';

  @override
  String get localModelsInstall => 'Installer';

  @override
  String get localModelsDeleteEngine => 'Supprimer le moteur';

  @override
  String get localModelsDownloadSource => 'Source de téléchargement';

  @override
  String get localModelsSourceHuggingFace => 'Hugging Face';

  @override
  String get localModelsSourceModelScope => 'ModelScope';

  @override
  String get localModelsSourceHfMirror => 'HF-Mirror';

  @override
  String get localModelsSourceDirectHttp => 'HTTP direct';

  @override
  String get localModelsExploreHub => 'Explorer / Importer des modèles';

  @override
  String get localModelsHubSearchHint =>
      'Rechercher des modèles ou saisir propriétaire/dépôt...';

  @override
  String get localModelsHubImport => 'Importer';

  @override
  String get localModelsHubImporting => 'Analyse et importation du modèle...';

  @override
  String get localModelsHubImportSuccess =>
      'Modèle importé dans le catalogue avec succès';

  @override
  String get localModelsRemoveFromCatalog => 'Retirer du catalogue';

  @override
  String get localModelsCustomBadge => 'Personnalisé';

  @override
  String get localModelsHubSearching => 'Recherche...';

  @override
  String get localModelsHubNoResults =>
      'Aucun dépôt de modèle correspondant trouvé';

  @override
  String get localModelsHubDirectImportTitle => 'Importer par dépôt';

  @override
  String get localModelsHubDirectImportPrompt =>
      'Saisissez un dépôt Hugging Face ou ModelScope (propriétaire/dépôt) ou une URL :';

  @override
  String get localModelDescriptionSherpaOnnxStreamingStt =>
      'Reconnaissance vocale bilingue chinois et anglais en streaming.';

  @override
  String get localModelDescriptionSherpaOnnxVitsAishell3 =>
      'Synthèse vocale chinoise locale multi-locuteurs.';

  @override
  String get localModelDescriptionSherpaOnnxVitsZhLl =>
      'Synthèse vocale chinoise locale à cinq locuteurs.';

  @override
  String get localModelDescriptionSherpaOnnxMatchaBaker =>
      'Synthèse vocale chinoise locale mono-locuteur Matcha.';

  @override
  String get localModelDescriptionSherpaOnnxKittenNano =>
      'Synthèse vocale anglaise locale à huit locuteurs KittenTTS.';

  @override
  String get localModelDescriptionSherpaOnnxWebParaformer =>
      'Reconnaissance vocale Paraformer chinois et anglais intégrée au navigateur.';

  @override
  String get localModelDescriptionSherpaOnnxWebVitsPiper =>
      'Synthèse vocale VITS anglaise multi-locuteurs intégrée au navigateur.';

  @override
  String get settingsModelProviderTypeMnn => 'MNN';

  @override
  String get settingsModelProviderTypeLlamaCpp => 'llama.cpp';

  @override
  String get settingsModelProviderTypePpinfra => 'PPInfra';

  @override
  String get settingsModelProviderTypeNovita => 'Novita AI';

  @override
  String get settingsModelProviderTypeMinimax => 'MiniMax';

  @override
  String get settingsModelProviderTypeOther => 'Autre';

  @override
  String get settingsModelEditModelSettings => 'Paramètres du modèle';

  @override
  String get settingsModelCreateProvider => 'Créer un fournisseur';

  @override
  String get settingsModelEditProvider => 'Modifier le fournisseur';

  @override
  String get settingsModelAddModel => 'Ajouter un modèle';

  @override
  String get settingsModelAddModelShort => 'Ajouter';

  @override
  String get settingsModelAvailableFetchedOnly => 'Récupérés uniquement';

  @override
  String get settingsModelAvailableIncludeHistory => 'Inclure l\'historique';

  @override
  String get settingsModelAvailableSourceFetched => 'Récupéré';

  @override
  String get settingsModelAvailableSourceHistory => 'Historique';

  @override
  String get settingsModelAvailableEmptyFetched =>
      'Aucun modèle récupéré. Choisissez Inclure l\'historique pour voir les ID du catalogue.';

  @override
  String get settingsModelUnrecognized => 'Non reconnu';

  @override
  String settingsModelCatalogUpdatedAt(String time) {
    return 'Capacités et contexte issus de models.dev, mis à jour : $time';
  }

  @override
  String settingsModelCatalogUpdateFailed(String error) {
    return 'Impossible de télécharger les détails du modèle : $error';
  }

  @override
  String settingsModelCatalogUpdateFailedUsing(String time, String error) {
    return 'Impossible de mettre à jour les détails du modèle, utilisation de ceux de $time : $error';
  }

  @override
  String get settingsModelCatalogBundled =>
      'Utilisation des détails intégrés du modèle ; les mises à jour en ligne les remplacent automatiquement';

  @override
  String get settingsModelCatalogRetry => 'Mettre à jour';

  @override
  String get settingsModelCustomModel => 'Modèle personnalisé';

  @override
  String get settingsModelModelId => 'ID du modèle';

  @override
  String get settingsModelDuplicateModelId =>
      'Ce modèle a déjà été ajouté à ce fournisseur.';

  @override
  String get settingsModelDuplicateProviderName =>
      'Un fournisseur portant ce nom existe déjà.';

  @override
  String get settingsModelMaxTokens => 'Tokens max';

  @override
  String get settingsModelMaxTokensDescription =>
      'Limite le nombre de Tokens qu\'une réponse peut générer.';

  @override
  String get settingsModelTemperature => 'Température';

  @override
  String get settingsModelTemperatureDescription =>
      'Contrôle le caractère aléatoire. Plus bas = plus stable, plus haut = plus varié.';

  @override
  String get settingsModelTopP => 'Top-p';

  @override
  String get settingsModelTopPDescription =>
      'Échantillonne uniquement dans la plage de probabilité cumulée Top-p.';

  @override
  String get settingsModelTopK => 'Top-k';

  @override
  String get settingsModelTopKDescription =>
      'Échantillonne parmi les K Tokens candidats les plus probables. 0 désactive l\'option.';

  @override
  String get settingsModelPresencePenalty => 'Pénalité de présence';

  @override
  String get settingsModelPresencePenaltyDescription =>
      'Encourage les nouveaux sujets et réduit la réutilisation du contenu existant.';

  @override
  String get settingsModelFrequencyPenalty => 'Pénalité de fréquence';

  @override
  String get settingsModelFrequencyPenaltyDescription =>
      'Pénalise les Tokens répétés selon leur fréquence.';

  @override
  String get settingsModelRepetitionPenalty => 'Pénalité de répétition';

  @override
  String get settingsModelRepetitionPenaltyDescription =>
      'Réduit davantage les répétitions. 1.0 signifie aucune pénalité.';

  @override
  String get settingsModelRequestLimit => 'Requêtes par minute';

  @override
  String get settingsModelMaxConcurrent => 'Requêtes simultanées max';

  @override
  String get settingsModelContextLength => 'Longueur du contexte';

  @override
  String get settingsModelMaxContextLength => 'Longueur max du contexte';

  @override
  String get settingsModelMaxContextLengthInvalid =>
      'Saisissez une longueur max du contexte supérieure à 0';

  @override
  String get settingsModelSummaryThreshold => 'Résumer à';

  @override
  String get settingsModelCustomHeaders => 'En-têtes personnalisés';

  @override
  String get settingsModelCustomParameters => 'Paramètres personnalisés JSON';

  @override
  String get settingsModelToolCall => 'Appel d\'outils';

  @override
  String get settingsModelToolCallDescription =>
      'Autorise le modèle à utiliser des appels d\'outils structurés.';

  @override
  String get settingsModelDirectImage => 'Entrée image directe';

  @override
  String get settingsModelDirectImageDescription =>
      'Envoie les images directement aux modèles prenant en charge l\'entrée image.';

  @override
  String get settingsModelDirectAudio => 'Entrée audio directe';

  @override
  String get settingsModelDirectAudioDescription =>
      'Envoie l\'audio directement aux modèles prenant en charge l\'entrée audio.';

  @override
  String get settingsModelDirectVideo => 'Entrée vidéo directe';

  @override
  String get settingsModelDirectVideoDescription =>
      'Envoie la vidéo directement aux modèles prenant en charge l\'entrée vidéo.';

  @override
  String get settingsModelGoogleSearch => 'Google Search';

  @override
  String get settingsModelGoogleSearchDescription =>
      'Active la fonction de recherche côté fournisseur.';

  @override
  String get settingsModelContext => 'Contexte';

  @override
  String get settingsModelSummary => 'Résumé automatique';

  @override
  String get settingsModelThinking => 'Réflexion';

  @override
  String get thinkingLevelLabel => 'Réflexion';

  @override
  String get thinkingAlwaysOn => 'Réflexion toujours active';

  @override
  String get thinkingChip => 'Réflexion';

  @override
  String get settingsModelThinkingSourceDeclared =>
      'Niveaux issus de votre description';

  @override
  String get settingsModelThinkingSourceLocalRuntime =>
      'Exécution locale : la réflexion s\'active ou se désactive, et l\'interrupteur est sans effet sur un modèle ne prenant pas en charge la réflexion';

  @override
  String get settingsModelThinkingUnknown =>
      'Le catalogue ne contient aucune information de réflexion pour ce modèle';

  @override
  String get settingsModelThinkingNotAdjustable =>
      'Cette API ne peut pas modifier le mode de réflexion de ce modèle';

  @override
  String get settingsModelThinkingNotSupported =>
      'Ce modèle ne prend pas en charge la réflexion';

  @override
  String get settingsModelThinkingCarried => 'Repris du dernier niveau défini';

  @override
  String get settingsModelThinkingDescribe => 'Décrire';

  @override
  String get settingsModelThinkingUseCatalog => 'Utiliser le catalogue';

  @override
  String get settingsModelThinkingDescribeTitle =>
      'Fonctionnement de la réflexion du modèle';

  @override
  String get settingsModelThinkingKindNone => 'Pas de réflexion';

  @override
  String get settingsModelThinkingKindAlways => 'Toujours active, sans réglage';

  @override
  String get settingsModelThinkingKindSwitch => 'Activée ou désactivée';

  @override
  String get settingsModelThinkingKindLevels => 'Niveaux';

  @override
  String get settingsModelThinkingKindBudget => 'Budget de Tokens';

  @override
  String get settingsModelThinkingCanTurnOff => 'Peut être désactivée';

  @override
  String get settingsModelThinkingBudgetMin => 'Tokens minimum';

  @override
  String get settingsModelThinkingBudgetMax => 'Tokens maximum';

  @override
  String get settingsModelThinkingLevelsRequired =>
      'Choisissez au moins un niveau';

  @override
  String get settingsModelMediaHistory => 'Historique média';

  @override
  String get settingsModelCapabilities => 'Capacités';

  @override
  String get settingsModelBuiltinTools => 'Outils intégrés';

  @override
  String get settingsModelBuiltinToolExclusive =>
      'Désactive l\'appel d\'outils externes lorsqu\'activé';

  @override
  String get settingsModelRunConnectionTest => 'Tester le modèle actuel';

  @override
  String get settingsModelTestModel => 'Tester';

  @override
  String get settingsModelCapabilityToolCall => 'Outils';

  @override
  String get settingsModelCapabilityImage => 'Image';

  @override
  String get settingsModelCapabilityAudio => 'Audio';

  @override
  String get settingsModelCapabilityVideo => 'Vidéo';

  @override
  String get settingsModelClaudeCache => 'Cache Claude 1 h';

  @override
  String get settingsModelClaudeCacheDescription =>
      'Définit un TTL d\'une heure sur les points de cache explicites ; désactivé, le TTL par défaut s\'applique.';

  @override
  String get settingsModelSummaryThresholdDescription =>
      'Résume lorsque le contexte atteint cette proportion. 0 désactive les résumés.';

  @override
  String get settingsModelMoreActions => 'Plus d\'actions';

  @override
  String get settingsModelRestoreDefaults => 'Rétablir les valeurs par défaut';

  @override
  String get settingsModelDescribeThinking =>
      'Décrire les niveaux de réflexion';

  @override
  String get settingsModelDeleteModel => 'Supprimer le modèle';

  @override
  String settingsModelSaveFailed(String error) {
    return 'Impossible d\'enregistrer : $error';
  }

  @override
  String get settingsModelTestAll => 'Tout tester';

  @override
  String settingsModelTestProgress(int done, int total) {
    return 'Test en cours $done/$total';
  }

  @override
  String settingsModelTestSummary(int passed, int failed) {
    return '$passed réussis, $failed échoués';
  }

  @override
  String settingsModelTestSummaryWithRestricted(
    int passed,
    int failed,
    int restricted,
  ) {
    return '$passed réussis, $failed échoués, $restricted restreints';
  }

  @override
  String settingsModelTestAllPassed(int count) {
    return 'Les $count tests ont réussi';
  }

  @override
  String get settingsModelTestCouldNotRun => 'Le test n\'a pas pu s\'exécuter';

  @override
  String get settingsModelCopyError => 'Copier l\'erreur';

  @override
  String get settingsModelSelect => 'Sélectionner';

  @override
  String get settingsModelCheckAll => 'Tout sélectionner';

  @override
  String settingsModelCheckedCount(int count) {
    return '$count sélectionné(s)';
  }

  @override
  String settingsModelDeleteModelsTitle(int count) {
    return 'Supprimer $count modèles ?';
  }

  @override
  String get settingsModelDeleteModelsMessage =>
      'Leurs paramètres seront également supprimés.';

  @override
  String settingsModelDeleteModelsKept(int count, String names) {
    return '$count utilisé(s) par des fonctions seront conservés : $names';
  }

  @override
  String settingsModelDeleteModelsBlocked(String functions) {
    return 'Les modèles sélectionnés sont utilisés par ces fonctions. Modifiez d\'abord l\'attribution des modèles : $functions';
  }

  @override
  String settingsModelDeletedCount(int count) {
    return '$count modèles supprimés';
  }

  @override
  String get settingsModelTestingConnection =>
      'Test de connexion du modèle actuel…';

  @override
  String get settingsModelTestedModel => 'Modèle testé';

  @override
  String get settingsModelConnectionTestPassed =>
      'Toutes les vérifications ont réussi';

  @override
  String get settingsModelConnectionTestFailed =>
      'Certaines vérifications ont échoué';

  @override
  String get settingsModelConnectionTestRestricted => 'Test restreint';

  @override
  String get settingsModelTestCreditRestricted =>
      'Limite de crédit ou de budget de sortie';

  @override
  String get settingsModelTestRateRestricted =>
      'Limite de débit ou délai d\'attente en amont';

  @override
  String get settingsModelCapabilitiesApplied =>
      'Les options de capacités du modèle ont été mises à jour selon le résultat du test.';

  @override
  String get settingsModelCapabilitiesNeedChat =>
      'Le test de discussion a échoué ; les options de capacités du modèle n\'ont pas été mises à jour.';

  @override
  String settingsModelConnectionTestError(String error) {
    return 'Échec du test de connexion : $error';
  }

  @override
  String get settingsModelTestItemChat => 'Discussion';

  @override
  String get settingsModelTestItemToolCall => 'Appel d\'outil';

  @override
  String get settingsModelTestItemImage => 'Image';

  @override
  String get settingsModelTestItemAudio => 'Audio';

  @override
  String get settingsModelTestItemVideo => 'Vidéo';

  @override
  String get settingsModelTestItemUnknown => 'Élément inconnu';

  @override
  String get settingsToolsPermissionMode => 'Mode de capacités IA';

  @override
  String get settingsToolsAsk => 'Demander';

  @override
  String get settingsToolsExtensions => 'Gestion des extensions';

  @override
  String get settingsToolsPlugins => 'Plugins';

  @override
  String get settingsToolsPluginsDescription =>
      'Gérer les conteneurs de plugins ToolPkg et les extensions d\'interface.';

  @override
  String get settingsToolsPackages => 'Paquets d\'outils';

  @override
  String get settingsToolsPackagesDescription =>
      'Activer, désactiver et inspecter les paquets d\'outils intégrés ou externes.';

  @override
  String get settingsToolsSkills => 'Skills';

  @override
  String get settingsToolsSkillsDescription =>
      'Gérer la visibilité et l\'importation des paquets de Skill.';

  @override
  String get settingsToolsMcp => 'Serveurs MCP';

  @override
  String settingsToolsMcpDescription(int seconds) {
    return 'Gérer la configuration MCP. Délai d\'attente au démarrage : $seconds secondes.';
  }

  @override
  String get settingsToolsOverrides => 'Enregistrements d\'outils';

  @override
  String get settingsToolsToolGroups => 'Outils enregistrés';

  @override
  String get settingsToolsToolGroupsDescription =>
      'Outils enregistrés par l\'environnement d\'exécution actuel pour l\'IA.';

  @override
  String get settingsToolsAlwaysAllow => 'Autorisés dans cette session';

  @override
  String get settingsToolsAlwaysAllowDescription =>
      'Ces outils ont été approuvés pour la session en cours.';

  @override
  String get settingsToolsAlwaysForbid => 'Toujours interdire';

  @override
  String get settingsToolsAlwaysForbidDescription =>
      'L\'IA n\'appellera pas ces outils.';

  @override
  String get settingsToolsAddTool => 'Ajouter un outil';

  @override
  String get settingsToolsAddAllowTool => 'Ajouter un outil autorisé';

  @override
  String get settingsToolsAddForbidTool => 'Ajouter un outil interdit';

  @override
  String get settingsToolsSearchTools => 'Rechercher des outils';

  @override
  String get settingsToolsNoToolsInGroup => 'Aucun outil dans ce groupe.';

  @override
  String get settingsToolsMcpStartupTimeout =>
      'Délai d\'expiration au démarrage MCP';

  @override
  String get settingsToolsMcpStartupTimeoutSeconds => 'Délai en secondes';

  @override
  String get settingsToolsToolPkgPreHookTimeout =>
      'Délai d\'expiration pré-hook ToolPkg';

  @override
  String settingsToolsToolPkgPreHookDescription(int seconds) {
    return 'Une chaîne pré-hook ToolPkg dispose de $seconds secondes au total.';
  }

  @override
  String get settingsToolsToolPkgPreHookTimeoutSeconds => 'Total en secondes';

  @override
  String get settingsWorkspaceCurrentDesign =>
      'Structure actuelle de l\'espace de travail';

  @override
  String get settingsWorkspaceCurrentDesignDescription =>
      'Les espaces de travail sont liés aux discussions. Les sessions de terminal et de navigateur sont globales et affichées à plat dans l\'espace de travail.';

  @override
  String get settingsWorkspaceOpenChat =>
      'Revenir à l\'espace de travail de discussion';

  @override
  String get settingsWorkspaceOpenChatDescription =>
      'Ouvrir les fichiers, terminaux, navigateur et automatisations web à droite de la discussion.';

  @override
  String get settingsWorkspaceContains => 'L\'espace de travail contient';

  @override
  String get settingsWorkspacePerChat => 'Lié par discussion';

  @override
  String get settingsWorkspaceGlobalSessions => 'Sessions de terminal globales';

  @override
  String get settingsWorkspaceBrowserSessions =>
      'Sessions de navigateur et WebVisit';

  @override
  String get settingsWorkspaceBoundOverview =>
      'Aperçu des liaisons d\'espaces de travail';

  @override
  String get settingsWorkspaceBoundOverviewDescription =>
      'Les chemins d\'espace de travail enregistrés dans l\'historique des discussions servent de source de liaison.';

  @override
  String get settingsWorkspaceBoundChats => 'Discussions liées';

  @override
  String get settingsWorkspaceInternalRoot =>
      'Racine interne des espaces de travail';

  @override
  String get settingsWorkspaceExternalRoot =>
      'Ancienne racine externe des espaces de travail';

  @override
  String get settingsWorkspaceUnboundTitle => 'Espaces de travail non liés';

  @override
  String get settingsWorkspaceUnboundSubtitle =>
      'Ces dossiers d\'espace de travail ne sont utilisés par aucune discussion.';

  @override
  String get settingsWorkspaceNoUnbound => 'Aucun espace de travail non lié.';

  @override
  String settingsWorkspaceSelectedCount(int selected, int total) {
    return 'Sélectionnés : $selected / $total';
  }

  @override
  String get settingsWorkspaceSelectAllCurrentList => 'Tout sélectionner';

  @override
  String get settingsWorkspaceClearAll => 'Effacer';

  @override
  String get settingsWorkspaceInternalStorage => 'Stockage interne';

  @override
  String get settingsWorkspaceExternalStorage => 'Stockage externe';

  @override
  String get settingsWorkspaceNotUsedByAnyChat =>
      'Non utilisé par une discussion';

  @override
  String settingsWorkspaceDeleteSelected(int count) {
    return 'Supprimer les espaces de travail sélectionnés ($count)';
  }

  @override
  String get settingsWorkspaceConfirmDeleteTitle => 'Confirmer la suppression';

  @override
  String settingsWorkspaceDeleteConfirmation(int count) {
    return 'Supprimer les $count dossiers d\'espace de travail sélectionnés ?';
  }

  @override
  String settingsWorkspaceDeleted(int count) {
    return '$count espaces de travail non liés supprimés.';
  }

  @override
  String settingsWorkspaceDeleteFailed(String error) {
    return 'Échec de la suppression : $error';
  }

  @override
  String settingsWorkspaceLoadFailed(String error) {
    return 'Échec du chargement des espaces de travail : $error';
  }

  @override
  String get settingsWorkspaceRefresh => 'Actualiser';

  @override
  String get runtimeIdentity => 'Identité actuelle';

  @override
  String get runtimeIdentityManage => 'Changer ou gérer les identités';

  @override
  String get runtimeIdentitySheetTitle => 'Identités';

  @override
  String get runtimeIdentityCreate => 'Nouvelle identité';

  @override
  String get runtimeIdentityCreateTitle => 'Nouvelle identité';

  @override
  String get runtimeIdentityRename => 'Renommer l\'identité';

  @override
  String get runtimeIdentityRenameTitle => 'Renommer l\'identité';

  @override
  String get runtimeIdentityName => 'Nom (facultatif)';

  @override
  String runtimeIdentitySwitchTitle(String identityName) {
    return 'Passer à $identityName ?';
  }

  @override
  String get runtimeIdentitySwitchDescription =>
      'Chaque identité dispose de ses propres discussions, paramètres, Space d\'appareil, appareils associés et espaces de travail. Changer d\'identité arrête l\'exécution en cours ; l\'identité sélectionnée sera utilisée au prochain démarrage de l\'application.';

  @override
  String get runtimeIdentitySwitchConfirm => 'Changer d\'identité';

  @override
  String get runtimeIdentityActivate => 'Basculer';

  @override
  String get runtimeIdentityActions => 'Actions pour cette identité';

  @override
  String get runtimeIdentityDelete => 'Supprimer l\'identité';

  @override
  String runtimeIdentityDeleteTitle(String identityName) {
    return 'Supprimer « $identityName » ?';
  }

  @override
  String get runtimeIdentityDeleteDescription =>
      'Cette identité sera retirée de la liste des identités de cet appareil. Ses fichiers locaux seront conservés.';

  @override
  String get runtimeIdentityCurrent => 'Actuelle';

  @override
  String get settingsUserProfileTitle => 'Gestion des identités';

  @override
  String get settingsUserProfileSubtitle => 'Avatar, nom, identités et GitHub';

  @override
  String get settingsUserProfileDescription =>
      'Gérez ce profil et basculez entre des identités isolées.';

  @override
  String get settingsUserProfileUnnamed => 'Sans nom';

  @override
  String get settingsUserProfileNotLoggedIn => 'Non connecté';

  @override
  String get settingsUserProfileGitHubLoading =>
      'Chargement du compte GitHub...';

  @override
  String settingsUserProfileGitHubAccount(String account) {
    return 'GitHub : @$account';
  }

  @override
  String settingsUserProfileGitHubStatusError(String error) {
    return 'Erreur de statut GitHub : $error';
  }

  @override
  String get settingsUserProfileOverview => 'Profil';

  @override
  String get settingsUserProfileAvatar => 'Avatar';

  @override
  String get settingsUserProfileName => 'Nom';

  @override
  String get settingsUserProfileChooseAvatar => 'Choisir un avatar';

  @override
  String get settingsUserProfileClearAvatar => 'Effacer l\'avatar';

  @override
  String get settingsUserProfileEditName => 'Modifier le nom';

  @override
  String get settingsUserProfileIdentities => 'Identités locales';

  @override
  String get settingsUserProfileGitHub => 'Compte GitHub';

  @override
  String get settingsUserProfileGitHubDescription => 'Non connecté';

  @override
  String get settingsUserProfileLogin => 'Se connecter';

  @override
  String get settingsUserProfileLogout => 'Se déconnecter';

  @override
  String get settingsAppearanceAvatarCustom => 'Avatar personnalisé';

  @override
  String get settingsRuntimeConnection => 'Space d\'appareils actuel';

  @override
  String get settingsRuntimeConnectionDescription =>
      'État de connexion de cet appareil et de son Space d\'appareils.';

  @override
  String get settingsRuntimeCurrentSpace => 'Space d\'appareils actuel';

  @override
  String get settingsRuntimeOverviewDescription =>
      'Les appareils d\'un même Space peuvent se connecter et partager données et routes.';

  @override
  String get settingsRuntimeRenameSpace => 'Renommer le Space d\'appareils';

  @override
  String get settingsRuntimeResetTransactions =>
      'Réinitialiser les transactions';

  @override
  String get settingsRuntimeResetTransactionsDone =>
      'Transactions de synchronisation reconstruites à partir du contenu actuel de cet appareil';

  @override
  String get settingsRuntimeLeaveSpace => 'Quitter le Space d\'appareils';

  @override
  String get settingsRuntimeLeaveSpaceTitle =>
      'Quitter le Space d\'appareils actuel ?';

  @override
  String get settingsRuntimeLeaveSpaceDescription =>
      'Cet appareil créera un nouveau Space individuel. Les données métier et les appareils associés sont conservés.';

  @override
  String get settingsRuntimeLeaveSpaceConfirm => 'Quitter';

  @override
  String get settingsRuntimeSpaceName => 'Nom du Space d\'appareils';

  @override
  String settingsRuntimeSpaceId(String spaceId) {
    return 'ID du Space d\'appareils : $spaceId';
  }

  @override
  String settingsRuntimeSpaceDeviceCount(int count) {
    return '$count appareils';
  }

  @override
  String get settingsRuntimeViewSpaceTopology =>
      'Voir la topologie des appareils';

  @override
  String settingsRuntimeSpaceTopologyTitle(String spaceName) {
    return 'Topologie des appareils de $spaceName';
  }

  @override
  String settingsRuntimeSpaceTopologySummary(
    int deviceCount,
    int connectionCount,
  ) {
    return '$deviceCount appareils · $connectionCount connexions directes';
  }

  @override
  String get settingsRuntimeDisconnectConnection => 'Déconnecter';

  @override
  String get settingsRuntimeDisconnectConnectionTitle =>
      'Couper la connexion directe';

  @override
  String settingsRuntimeDisconnectConnectionMessage(String deviceName) {
    return 'Couper la connexion directe avec $deviceName ? Les enregistrements d\'association seront conservés.';
  }

  @override
  String get settingsRuntimeDisconnectConnectionFailed =>
      'Échec de la déconnexion';

  @override
  String get settingsRuntimeCurrentDevice => 'Appareil actuel';

  @override
  String get settingsRuntimeRemoteTitle => 'Appareils connectés';

  @override
  String get settingsRuntimeRemoteDescription =>
      'Appareils associés directement. L\'association établit une connexion ; rejoindre un Space d\'appareils active le partage de données et le routage.';

  @override
  String get settingsRuntimePairRemote => 'Connecter un autre appareil';

  @override
  String get settingsRuntimeDiscoveredEdges => 'Appareils ESP32';

  @override
  String get settingsRuntimeEdgeEndpointRequired =>
      'Le point de terminaison Edge est requis';

  @override
  String get settingsRuntimePairingCodeRequired =>
      'Le code d\'association est requis';

  @override
  String get settingsRuntimeNoPairedRemote =>
      'Aucun appareil connecté pour le moment.';

  @override
  String get settingsRuntimeConnectionInitiatedByOtherDevice =>
      'Connexion initiée par l\'autre appareil';

  @override
  String get settingsRuntimePairCode => 'Code d\'association';

  @override
  String get settingsRuntimeStartPairing => 'Démarrer la connexion';

  @override
  String get settingsRuntimeFinishPairing => 'Terminer la connexion';

  @override
  String get settingsRuntimeBaseUrl => 'Adresse de l\'appareil';

  @override
  String get settingsRuntimeTcpAddressHint =>
      'hôte:port, par exemple 192.168.1.10:37195';

  @override
  String get settingsRuntimeTransportAuto => 'Auto (recommandé)';

  @override
  String get settingsRuntimeTransportAutoHint =>
      'Saisissez hôte:port ; le protocole (HTTP/WebSocket/TCP) est détecté automatiquement';

  @override
  String settingsRuntimeConnectionFailed(String error) {
    return 'Échec de connexion à l\'appareil : $error';
  }

  @override
  String get settingsRuntimePairingRejected =>
      'Une connexion d\'appareil a été refusée';

  @override
  String get settingsRuntimePairedChecking => 'Vérification';

  @override
  String get settingsRuntimePairedOnline => 'En ligne';

  @override
  String get settingsRuntimePairedOffline => 'Hors ligne';

  @override
  String get settingsRuntimePairedInvalid => 'Association invalide';

  @override
  String get settingsRuntimePairedError => 'Statut indisponible';

  @override
  String get settingsRuntimePairingRevokedTitle =>
      'Association révoquée par l\'autre appareil';

  @override
  String get settingsRuntimePairingRevokedMessage =>
      'L\'autre appareil a annulé son association avec cet appareil. L\'enregistrement local n\'est plus valide.';

  @override
  String get settingsRuntimePairingRevokedConfirm => 'Compris';

  @override
  String get settingsRuntimePairedRemovedFromSpace =>
      'Retiré du Space d\'appareils';

  @override
  String get settingsRuntimeRemovedFromSpaceTitle =>
      'Retiré du Space d\'appareils';

  @override
  String get settingsRuntimeRemovedFromSpaceMessage =>
      'L\'appareil distant a retiré cet appareil du Space. Quittez le Space actuel et créez un Space autonome maintenant.';

  @override
  String get settingsRuntimeRemovedFromSpaceConfirm =>
      'Quitter et créer un Space autonome';

  @override
  String get settingsRuntimeJoinSpace => 'Rejoindre le Space d\'appareils';

  @override
  String settingsRuntimeJoinSpaceTitle(String deviceName) {
    return 'Rejoindre le Space d\'appareils de $deviceName ?';
  }

  @override
  String get settingsRuntimeJoinSpaceDescription =>
      'Le Space d\'appareils actuel fusionnera avec l\'autre et reprendra son nom.';

  @override
  String get settingsRuntimePairingComplete => 'Appareil associé';

  @override
  String get settingsRuntimeDeviceInCurrentSpace =>
      'Dans le Space d\'appareils actuel';

  @override
  String get settingsRuntimeDiscoverSpaces =>
      'Découvrir des Spaces d\'appareils';

  @override
  String get settingsRuntimeDiscoverSpacesDescription =>
      'Les appareils à proximité sont regroupés par Space d\'appareils. Développez un Space pour vous connecter directement à l\'un de ses appareils.';

  @override
  String get settingsRuntimeWebPairDescription =>
      'Saisissez l\'adresse d\'un autre appareil et confirmez le code d\'association pour connecter ce navigateur en tant que client associé actif.';

  @override
  String settingsRuntimeDiscoveredSpaceSummary(
    int memberCount,
    int nearbyCount,
  ) {
    return '$memberCount appareils au total · $nearbyCount à proximité';
  }

  @override
  String get settingsRuntimeScan => 'Rechercher';

  @override
  String get settingsRuntimeScanning => 'Recherche…';

  @override
  String get settingsRuntimeEnterManually => 'Saisir manuellement';

  @override
  String get settingsRuntimeConnect => 'Connecter';

  @override
  String get settingsRuntimeEnableDiscovery =>
      'Autoriser les appareils à proximité à découvrir ce Space d\'appareils';

  @override
  String get settingsRuntimeEnableDiscoveryDescription =>
      'Les appareils sur le même réseau peuvent trouver ce Space d\'appareils et choisir cet appareil pour une connexion directe.';

  @override
  String settingsRuntimeEnableDiscoveryFailed(String error) {
    return 'Impossible d\'activer la découverte du Space d\'appareils : $error';
  }

  @override
  String settingsRuntimeDisableDiscoveryFailed(String error) {
    return 'Impossible de désactiver la découverte du Space d\'appareils : $error';
  }

  @override
  String get settingsRuntimeUsingLocal => 'Actif : cet appareil';

  @override
  String settingsRuntimeUsingRemote(String device) {
    return 'Actif : $device';
  }

  @override
  String get settingsRuntimeRemoteInUseDescription =>
      'Les discussions et les outils s\'exécutent sur cet appareil connecté.';

  @override
  String get settingsWebAccessService => 'Autoriser l\'accès';

  @override
  String get settingsWebAccessServiceDescription =>
      'Une fois activé, les navigateurs peuvent accéder à cet appareil avec une adresse et un token.';

  @override
  String get settingsWebAccessEnable => 'Autoriser l\'accès externe';

  @override
  String get settingsWebAccessPortMode => 'Mode de port';

  @override
  String get settingsWebAccessPortAutomatic => 'Automatique';

  @override
  String get settingsWebAccessPortFixed => 'Fixe';

  @override
  String get settingsWebAccessPortAutomaticDescription =>
      'L\'application choisit un port automatiquement. Aucune configuration manuelle n\'est requise.';

  @override
  String get settingsWebAccessPortFixedDescription =>
      'Seul le port de l\'adresse d\'écoute est utilisé.';

  @override
  String get settingsWebAccessBindAddress => 'Adresse d\'écoute';

  @override
  String get settingsWebAccessToken => 'Token d\'accès';

  @override
  String get settingsWebAccessRotateToken => 'Changer le token';

  @override
  String get settingsWebAccessCopyToken => 'Copier le token';

  @override
  String get settingsWebAccessAccessUrl => 'Adresse d\'accès';

  @override
  String get settingsWebAccessLocalUrl => 'Cet appareil';

  @override
  String get settingsWebAccessPairingUrl => 'Adresse d\'association';

  @override
  String get settingsWebAccessPairingUrlLocalOnly => 'Cet appareil uniquement';

  @override
  String get settingsWebAccessPairingUrlUnavailable =>
      'Aucune adresse LAN trouvée';

  @override
  String get settingsWebAccessCopyUrl => 'Copier l\'URL';

  @override
  String get settingsWebAccessOpenUrl => 'Ouvrir l\'adresse';

  @override
  String get settingsWebAccessRunning => 'Activé';

  @override
  String get settingsWebAccessStopped => 'Désactivé';

  @override
  String get settingsWebAccessSaved => 'Paramètres d\'accès enregistrés.';

  @override
  String get settingsWebAccessTokenCopied => 'Token d\'accès copié.';

  @override
  String get settingsWebAccessUrlCopied => 'URL d\'accès copiée.';

  @override
  String get settingsWebAccessPairedClients => 'Appareils autorisés';

  @override
  String get settingsWebAccessNoPairedClients =>
      'Aucun appareil autorisé pour le moment.';

  @override
  String get settingsWebAccessPairedDeleted => 'Appareil autorisé supprimé.';

  @override
  String get settingsWebAccessPairingRequest => 'Demande d\'association';

  @override
  String settingsWebAccessPairingRequestMessage(String code, String client) {
    return 'Code d\'association : $code\nAppareil : $client';
  }

  @override
  String get settingsWebAccessInvalidBindAddress =>
      'L\'adresse d\'écoute doit être au format hôte:port.';

  @override
  String settingsWebAccessStartFailed(String error) {
    return 'Impossible d\'activer l\'accès : $error';
  }

  @override
  String settingsWebAccessStopFailed(String error) {
    return 'Impossible de désactiver l\'accès : $error';
  }

  @override
  String get settingsAppearanceThemeSection => 'Thème';

  @override
  String get settingsAppearanceThemeMode => 'Mode actuel';

  @override
  String get settingsAppearanceThemeSystem => 'Système';

  @override
  String get settingsAppearanceThemeLight => 'Clair';

  @override
  String get settingsAppearanceThemeDark => 'Sombre';

  @override
  String get settingsAppearanceInputSection => 'Saisie';

  @override
  String get settingsAppearanceInputStyle => 'Style de saisie';

  @override
  String get settingsAppearanceInputStyleClassic => 'Classique';

  @override
  String get settingsAppearanceInputStyleAgent => 'Agent';

  @override
  String get settingsAppearanceInputFloating => 'Saisie flottante';

  @override
  String get settingsAppearanceColorSection => 'Couleur du thème';

  @override
  String get settingsAppearanceColorDescription =>
      'Choisissez un préréglage de couleur simple. Les barres système et l\'interface de l\'application suivent automatiquement le thème.';

  @override
  String get settingsAppearanceColorDefault => 'Par défaut';

  @override
  String get settingsAppearanceColorSky => 'Ciel';

  @override
  String get settingsAppearanceColorMatcha => 'Matcha';

  @override
  String get settingsAppearanceColorEmber => 'Braise';

  @override
  String get settingsAppearanceColorRose => 'Rose';

  @override
  String get settingsAppearanceColorCustom => 'Couleurs personnalisées';

  @override
  String get settingsAppearanceCustomColorsTitle =>
      'Couleurs de thème personnalisées';

  @override
  String get settingsAppearancePrimaryColor => 'Couleur principale';

  @override
  String get settingsAppearanceSecondaryColor => 'Couleur secondaire';

  @override
  String get settingsAppearanceHexColorHint => '#RRGGBB';

  @override
  String get settingsAppearanceHexColorInvalid =>
      'Saisissez une couleur au format #RRGGBB';

  @override
  String get settingsAppearanceBackgroundSection => 'Arrière-plan';

  @override
  String get settingsAppearanceBackgroundDescription =>
      'Choisissez une image ou une vidéo locale comme arrière-plan. Les surfaces de l\'application et les barres système suivent automatiquement le thème.';

  @override
  String get settingsAppearanceBackgroundImage => 'Média d\'arrière-plan';

  @override
  String get settingsAppearanceBackgroundNone => 'Aucun sélectionné';

  @override
  String get settingsAppearanceBackgroundChooseImage => 'Choisir une image';

  @override
  String get settingsAppearanceBackgroundChooseVideo => 'Choisir une vidéo';

  @override
  String get settingsAppearanceBackgroundDisable =>
      'Désactiver l\'arrière-plan';

  @override
  String get settingsAppearanceBackgroundEnabled => 'Activer l\'arrière-plan';

  @override
  String get settingsAppearanceBackgroundOpacity =>
      'Opacité de l\'arrière-plan';

  @override
  String get settingsAppearanceBackgroundBlur => 'Flouter l\'arrière-plan';

  @override
  String get settingsAppearanceBackgroundBlurRadius => 'Intensité du flou';

  @override
  String get settingsAppearanceBackgroundVideoMuted =>
      'Couper le son de la vidéo d\'arrière-plan';

  @override
  String get settingsAppearanceBackgroundVideoLoop =>
      'Lire la vidéo d\'arrière-plan en boucle';

  @override
  String get settingsAppearanceTextSection => 'Texte';

  @override
  String get settingsAppearanceFontFamily => 'Police';

  @override
  String get settingsAppearanceFontDefault => 'Par défaut';

  @override
  String get settingsAppearanceCustomFont => 'Police personnalisée';

  @override
  String get settingsAppearanceFontCustom => 'Personnalisée';

  @override
  String get settingsAppearanceChooseCustomFont =>
      'Choisir une police personnalisée';

  @override
  String get settingsAppearanceClearCustomFont =>
      'Effacer la police personnalisée';

  @override
  String get settingsAppearanceFontSerif => 'Serif';

  @override
  String get settingsAppearanceFontMonospace => 'Mono';

  @override
  String get settingsAppearanceNormalFontSection => 'Body font';

  @override
  String get settingsAppearanceFontsSection => 'Fonts';

  @override
  String get settingsAppearanceCodeFontSection => 'Code font';

  @override
  String get settingsAppearanceCodeFontDescription =>
      'Used by code blocks, inline code, diffs, file previews and the terminal';

  @override
  String get settingsAppearanceCodeFontBuiltin => 'Built-in mono';

  @override
  String get settingsAppearanceChatBodyFontSection => 'Chat text font';

  @override
  String get settingsAppearanceChatBodyFontFollow => 'Follows the body font';

  @override
  String get settingsAppearanceChatBodySplitSides =>
      'Set user and AI separately';

  @override
  String get settingsAppearanceChatBodyAdjust => 'Adjust';

  @override
  String get settingsAppearanceInterfaceScale => 'Échelle de l\'interface';

  @override
  String get settingsAppearanceFontScale => 'Taille de police';

  @override
  String get settingsAppearanceAvatarSection => 'Avatars';

  @override
  String get settingsAppearanceUserAvatar => 'Avatar utilisateur';

  @override
  String get settingsAppearanceAiAvatar => 'Avatar IA';

  @override
  String get settingsAppearanceAvatarDefault => 'Avatar par défaut';

  @override
  String get settingsAppearanceAvatarShape => 'Forme des avatars';

  @override
  String get settingsAppearanceAvatarShapeCircle => 'Cercle';

  @override
  String get settingsAppearanceAvatarShapeSquare => 'Carré';

  @override
  String get settingsAppearanceChooseUserAvatar =>
      'Choisir l\'avatar utilisateur';

  @override
  String get settingsAppearanceChooseAiAvatar => 'Choisir l\'avatar IA';

  @override
  String get settingsAppearanceClearUserAvatar =>
      'Effacer l\'avatar utilisateur';

  @override
  String get settingsAppearanceClearAiAvatar => 'Effacer l\'avatar IA';

  @override
  String get settingsAppearanceChatDisplaySection => 'Affichage du chat';

  @override
  String get settingsAppearanceUserMessageStyle =>
      'Style des messages utilisateur';

  @override
  String get settingsAppearanceAiMessageStyle => 'Style des messages IA';

  @override
  String get settingsAppearanceMessageStyleClean => 'Commande';

  @override
  String get settingsAppearanceMessageStyleCard => 'Bulle';

  @override
  String get settingsAppearanceMessageColors => 'Couleurs des messages';

  @override
  String get settingsAppearanceMessageColorsTheme => 'Suivre le thème';

  @override
  String get settingsAppearanceMessageColorsSky => 'Bleu clair';

  @override
  String get settingsAppearanceMessageColorsMatcha => 'Matcha';

  @override
  String get settingsAppearanceMessageColorsInk => 'Sombre';

  @override
  String get settingsAppearanceMessageColorsCustom =>
      'Couleurs de messages personnalisées';

  @override
  String get settingsAppearanceCustomMessageColorsTitle =>
      'Couleurs de messages personnalisées';

  @override
  String get settingsAppearanceCursorUserBubbleColor =>
      'Bulle utilisateur (Commande)';

  @override
  String get settingsAppearanceUserBubbleColor => 'Bulle utilisateur';

  @override
  String get settingsAppearanceAiBubbleColor => 'Bulle IA';

  @override
  String get settingsAppearanceUserTextColor => 'Texte utilisateur';

  @override
  String get settingsAppearanceAiTextColor => 'Texte IA';

  @override
  String get settingsAppearanceMessageSurface => 'Texture globale';

  @override
  String get settingsAppearanceMessageSurfaceNormal => 'Normale';

  @override
  String get settingsAppearanceMessageSurfaceTransparent => 'Transparente';

  @override
  String get settingsAppearanceUserBubbleFont => 'Police de bulle utilisateur';

  @override
  String get settingsAppearanceAiBubbleFont => 'Police de bulle IA';

  @override
  String get settingsAppearanceAdjustUserBubbleFont =>
      'Ajuster la police de bulle utilisateur';

  @override
  String get settingsAppearanceAdjustAiBubbleFont =>
      'Ajuster la police de bulle IA';

  @override
  String get settingsAppearanceEnableBubbleFont =>
      'Activer une police propre aux bulles';

  @override
  String get settingsAppearanceUserBubbleImage => 'Image de bulle utilisateur';

  @override
  String get settingsAppearanceAiBubbleImage => 'Image de bulle IA';

  @override
  String get settingsAppearanceChooseUserBubbleImage =>
      'Choisir la bulle utilisateur';

  @override
  String get settingsAppearanceChooseAiBubbleImage => 'Choisir la bulle IA';

  @override
  String get settingsAppearanceClearUserBubbleImage =>
      'Effacer la bulle utilisateur';

  @override
  String get settingsAppearanceClearAiBubbleImage => 'Effacer la bulle IA';

  @override
  String get settingsAppearanceBubbleImageRenderMode =>
      'Mode d\'image de bulle';

  @override
  String get settingsAppearanceBubbleImageTiledNineSlice => 'Mosaïque 9-slice';

  @override
  String get settingsAppearanceBubbleImageNinePatch => 'Étirement 9-patch';

  @override
  String get settingsAppearanceBubbleImageAdjustUser =>
      'Ajuster l\'image de bulle utilisateur';

  @override
  String get settingsAppearanceBubbleImageAdjustAi =>
      'Ajuster l\'image de bulle IA';

  @override
  String get settingsAppearanceBubbleImagePreview => 'Aperçu en direct';

  @override
  String get settingsAppearanceBubbleImagePreviewText =>
      'Ceci est un message d\'aperçu de bulle.\nVérifiez les coins, les bords et le centre à leur taille réelle.';

  @override
  String get settingsAppearanceBubbleImageSource => 'Original';

  @override
  String get settingsAppearanceBubbleImageSourceHint =>
      'Original · Blanc : zone de recadrage. Bleu : grille 9-slice. Pincez pour zoomer et faites glisser pour inspecter.';

  @override
  String get settingsAppearanceBubbleImageFitSource =>
      'Ajuster l\'image d\'origine';

  @override
  String get settingsAppearanceBubbleImageGrid => '9-slice';

  @override
  String get settingsAppearanceBubbleImageGridHint =>
      'Deux coupes horizontales et deux verticales forment neuf zones. Les coins restent fixes ; les bords se répètent sur un axe et le centre sur les deux. Le mode étirement étire ces zones à la place. Les limites sont relatives à l\'image recadrée.';

  @override
  String settingsAppearanceBubbleImageImportFailed(String error) {
    return 'Image de bulle non appliquée : $error';
  }

  @override
  String get settingsAppearanceBubbleImageCrop => 'Recadrer';

  @override
  String get settingsAppearanceBubbleImageRepeat => 'Zone de répétition';

  @override
  String get settingsAppearanceBubbleImageScale => 'Échelle de l\'image';

  @override
  String get settingsAppearanceBubbleImageCropLeft => 'Recadrage gauche';

  @override
  String get settingsAppearanceBubbleImageCropTop => 'Recadrage haut';

  @override
  String get settingsAppearanceBubbleImageCropRight => 'Recadrage droite';

  @override
  String get settingsAppearanceBubbleImageCropBottom => 'Recadrage bas';

  @override
  String get settingsAppearanceBubbleImageRepeatStart => 'Début répétition X';

  @override
  String get settingsAppearanceBubbleImageRepeatEnd => 'Fin répétition X';

  @override
  String get settingsAppearanceBubbleImageRepeatYStart => 'Début répétition Y';

  @override
  String get settingsAppearanceBubbleImageRepeatYEnd => 'Fin répétition Y';

  @override
  String get settingsAppearanceMessageDensity => 'Espacement des messages';

  @override
  String get settingsAppearanceMessageDensityComfortable => 'Confortable';

  @override
  String get settingsAppearanceMessageDensityCompact => 'Compact';

  @override
  String get settingsAppearanceChatHorizontalPadding =>
      'Chat content side padding';

  @override
  String get settingsAppearanceChatHorizontalPaddingEdge => 'Edge to edge';

  @override
  String get settingsAppearanceChatHorizontalPaddingDetail =>
      'Smaller values widen the transcript; bubble and composer insets are unaffected.';

  @override
  String get settingsAppearanceWideLayout =>
      'Utiliser une mise en page de chat plus large';

  @override
  String get settingsAppearanceRoundedMessages => 'Cartes de message arrondies';

  @override
  String get settingsAppearanceShowAvatars =>
      'Afficher les avatars des messages';

  @override
  String get settingsAppearanceMessageDisplaySection =>
      'Affichage des messages';

  @override
  String get settingsAppearanceShowThinkingProcess =>
      'Afficher le processus de réflexion';

  @override
  String get settingsAppearanceShowRoleName =>
      'Afficher les noms personnalisés des répondants';

  @override
  String get settingsAppearanceShowUserName => 'Afficher le nom d\'utilisateur';

  @override
  String get settingsAppearanceShowModelName => 'Afficher le nom du modèle';

  @override
  String get settingsAppearanceShowModelProvider =>
      'Afficher le fournisseur du modèle';

  @override
  String get settingsAppearanceShowMessageTokenStats =>
      'Afficher les statistiques de tokens';

  @override
  String get settingsAppearanceShowMessageTimingStats =>
      'Afficher les statistiques de durée';

  @override
  String get settingsAppearanceShowMessageTimestamp =>
      'Afficher l\'heure des messages';

  @override
  String get settingsAppearanceShowInputProcessingStatus =>
      'Afficher l\'état de traitement de la saisie';

  @override
  String get settingsAppearanceResetTheme =>
      'Réinitialiser les paramètres du thème';

  @override
  String get settingsAppLanguageTitle => 'Langue de l’application';

  @override
  String get settingsAppLanguageSystem => 'Suivre le système';

  @override
  String get settingsAppLanguageDescription =>
      'Les modifications s’appliquent immédiatement, sans redémarrage.';

  @override
  String settingsAppLanguageSaveError(String error) {
    return 'Impossible d’enregistrer la langue : $error';
  }

  @override
  String get settingsAppearanceBubblesTab => 'Messages et bulles';

  @override
  String get settingsAppearanceInteractionTab => 'Interaction et flux';

  @override
  String get settingsAppearanceBubbleStyleSection => 'Mise en page et style';

  @override
  String get settingsAppearanceBubbleFontSection => 'Polices des bulles';

  @override
  String get settingsAppearanceBubbleImageSection => 'Images des bulles';

  @override
  String get settingsAppearanceInteractionStatusSection =>
      'Processus et statut';

  @override
  String get settingsAppearanceBackgroundEffectsSection => 'Effets visuels';

  @override
  String get settingsAppearanceBackgroundVideoSection => 'Lecture vidéo';

  @override
  String get settingsAppearanceLivePreviewTitle => 'Aperçu en direct';

  @override
  String get settingsAppearanceLivePreviewUserSample =>
      'Aide-moi à organiser mon plan de travail d\'aujourd\'hui.';

  @override
  String get settingsAppearanceLivePreviewAiSample =>
      'Bien sûr ! J\'ai organisé votre liste de tâches principales et nous sommes prêts à commencer.';

  @override
  String get settingsAppearanceLivePreviewThinkingSample =>
      'Analyse du contexte et des priorités du planning...';

  @override
  String get settingsDataLogsSection => 'Journaux de diagnostic';

  @override
  String get settingsDataLogsDescription =>
      'Exportez les journaux du Core sélectionné et de ce client dans un seul fichier .log trié chronologiquement, marqué [CORE] / [CLIENT]. Les journaux peuvent contenir des données sensibles ; vérifiez-les avant de les partager.';

  @override
  String get settingsDataExportLogs => 'Exporter les journaux de diagnostic';

  @override
  String get settingsDataLogsExporting => 'Exportation des journaux…';

  @override
  String settingsDataLogsExportError(String error) {
    return 'Échec de l\'exportation des journaux : $error';
  }

  @override
  String settingsDataLogsExportPartial(String path) {
    return 'Enregistré dans $path. Une source de journaux n\'a pas pu être lue ; consultez les avertissements dans le fichier.';
  }

  @override
  String settingsDataOverviewLoadError(String error) {
    return 'Impossible de charger l\'aperçu des données : $error. Vous pouvez tout de même exporter les journaux de diagnostic.';
  }

  @override
  String get settingsDataRuntimeSection => 'Aperçu des données';

  @override
  String get settingsDataCoreVersion => 'Version actuelle';

  @override
  String get settingsDataStorageSection => 'Emplacement de stockage';

  @override
  String get settingsDataStorageDescription =>
      'Déplacez les données du runtime et de l\'espace de travail dans des dossiers locaux sélectionnés séparément.';

  @override
  String get settingsDataRuntimeRoot => 'Racine du runtime';

  @override
  String get settingsDataWorkspaceRoot => 'Racine de l\'espace de travail';

  @override
  String get settingsDataChooseStorageRoots => 'Modifier les emplacements';

  @override
  String get settingsDataEditStorageRootsTitle =>
      'Modifier les emplacements de stockage';

  @override
  String get settingsDataStorageRootsRequired =>
      'La racine du runtime et la racine de l\'espace de travail sont obligatoires.';

  @override
  String get storageDirectoryTermuxUnsupported =>
      'Le fournisseur de documents de Termux n\'est pas pris en charge pour les chemins de stockage local. Choisissez un dossier dans le stockage partagé (tel que Download ou Documents) plutôt que le dossier privé de Termux.';

  @override
  String get storageDirectoryProviderUnsupported =>
      'Le dossier sélectionné ne correspond pas à un chemin de stockage local pris en charge. Choisissez un dossier dans le stockage partagé (tel que Download ou Documents) ou saisissez un chemin local accessible par cette application.';

  @override
  String storageDirectorySelectionFailed(String error) {
    return 'Impossible de sélectionner un dossier : $error';
  }

  @override
  String get settingsDataStorageConfirmTitle =>
      'Changer l\'emplacement de stockage';

  @override
  String get settingsDataStorageConfirmMessage =>
      'Les données du runtime et de l\'espace de travail seront copiées dans les dossiers sélectionnés, et l\'application les utilisera après redémarrage.';

  @override
  String get settingsDataStorageConfirmAction => 'Changer l\'emplacement';

  @override
  String get settingsDataStorageChanged =>
      'Emplacement de stockage modifié. Redémarrez l\'application pour l\'utiliser.';

  @override
  String settingsDataStorageChangeError(String error) {
    return 'Échec du changement d\'emplacement de stockage : $error';
  }

  @override
  String get settingsDataInputTokens => 'Entrée';

  @override
  String get settingsDataOutputTokens => 'Sortie';

  @override
  String get settingsDataResetTokenStats => 'Réinitialiser les statistiques';

  @override
  String get settingsUsageReportResetConfirmation =>
      'Cela supprime définitivement tous les relevés d\'utilisation des modèles enregistrés. Vos conversations sont conservées.';

  @override
  String get settingsDataDetailedStatsDescription =>
      'Les statistiques sont calculées à partir des relevés dédiés aux requêtes de modèle.';

  @override
  String get settingsDataDetailedStatsEmpty =>
      'Aucun relevé d\'utilisation détaillé pour le moment';

  @override
  String settingsDataDetailedStatsDateRange(String start, String end) {
    return 'Du $start au $end';
  }

  @override
  String get settingsDataDetailedStatsSourceLabel =>
      'Relevés de requêtes de modèle';

  @override
  String get settingsDataDetailedStatsHistoricalTotal => 'Total historique';

  @override
  String get settingsDataDetailedStatsRangeAll => 'Tout';

  @override
  String get settingsDataDetailedStatsRangeLast7Days => '7 jours';

  @override
  String get settingsDataDetailedStatsRangeLast30Days => '30 jours';

  @override
  String get settingsDataDetailedStatsRangeCustom => 'Personnalisé';

  @override
  String get settingsDataDetailedStatsCustomRangePickerTitle =>
      'Sélectionner la plage de dates des statistiques';

  @override
  String get settingsDataDetailedStatsNoDataInRange =>
      'Aucune donnée sur cette période';

  @override
  String get settingsDataDetailedStatsNoRankRows =>
      'Aucune donnée de classement sur cette période';

  @override
  String get settingsDataDetailedStatsFunctionModels => 'Modèles de fonction';

  @override
  String get settingsDataDetailedStatsFunctionModelPieTitle =>
      'Répartition des modèles de fonction';

  @override
  String get settingsDataDetailedStatsTopFunctionModelsTitle =>
      'Classement des modèles de fonction';

  @override
  String get settingsDataDetailedStatsTopFunctionModelsSubtitle =>
      'Plus forte consommation totale de tokens par modèle de fonction';

  @override
  String get settingsDataDetailedStatsSourceChat => 'Réponse de discussion';

  @override
  String get settingsDataDetailedStatsSourceToolResult =>
      'Réponse de résultat d\'outil';

  @override
  String get settingsDataDetailedStatsSourceSummary => 'Génération de résumé';

  @override
  String get settingsDataDetailedStatsSourceTitleGeneration =>
      'Génération de titre';

  @override
  String get settingsDataDetailedStatsTotalRequests => 'Total des requêtes';

  @override
  String get settingsDataDetailedStatsCachedInput => 'Entrée en cache';

  @override
  String get settingsDataDetailedStatsActiveDays => 'Jours actifs';

  @override
  String get settingsDataDetailedStatsChats => 'Conversations';

  @override
  String get settingsDataDetailedStatsProviders => 'Fournisseurs';

  @override
  String get settingsDataDetailedStatsModels => 'Modèles';

  @override
  String get settingsDataDetailedStatsUnknownTokenRecords =>
      'Relevés avec tokens inconnus';

  @override
  String get settingsDataDetailedStatsHeatmapTitle =>
      'Carte thermique d\'utilisation';

  @override
  String get settingsDataDetailedStatsHeatmapSubtitle =>
      'Intensité quotidienne des tokens sur la période sélectionnée';

  @override
  String get settingsDataDetailedStatsHeatmapTapHint =>
      'Touchez une cellule pour afficher les détails';

  @override
  String settingsDataDetailedStatsHeatmapDayDetail(
    String date,
    String tokens,
    String requests,
  ) {
    return '$date : $tokens tokens utilisés · $requests requêtes';
  }

  @override
  String get settingsDataDetailedStatsHeatmapLow => 'Moins';

  @override
  String get settingsDataDetailedStatsHeatmapHigh => 'Plus';

  @override
  String get settingsDataDetailedStatsDailyUsageTitle =>
      'Tendance d\'utilisation quotidienne';

  @override
  String get settingsDataDetailedStatsDailyUsageSubtitle =>
      'Nombre de requêtes par jour';

  @override
  String get settingsDataDetailedStatsRequestsSeries => 'Requêtes';

  @override
  String get settingsDataDetailedStatsInputOutputTitle =>
      'Tendance de consommation entrée / sortie';

  @override
  String get settingsDataDetailedStatsInputOutputSubtitle =>
      'Évolution quotidienne des tokens en entrée et en sortie';

  @override
  String get settingsDataDetailedStatsProviderPieTitle =>
      'Répartition par fournisseur';

  @override
  String get settingsDataDetailedStatsModelPieTitle => 'Répartition par modèle';

  @override
  String get settingsDataDetailedStatsChatPieTitle =>
      'Répartition par conversation';

  @override
  String get settingsDataDetailedStatsTotalTokens => 'Total des tokens';

  @override
  String get settingsDataDetailedStatsTopRequestsTitle => 'Pics par requête';

  @override
  String get settingsDataDetailedStatsTopRequestsSubtitle =>
      'Plus forte consommation de tokens sur des appels individuels';

  @override
  String get settingsDataDetailedStatsTopChatsTitle =>
      'Principales conversations';

  @override
  String get settingsDataDetailedStatsTopChatsSubtitle =>
      'Plus forte consommation totale de tokens par conversation';

  @override
  String get settingsDataDetailedStatsOther => 'Autres';

  @override
  String settingsDataDetailedStatsInputOutputSummary(
    String input,
    String output,
    String chatTitle,
    String time,
  ) {
    return 'Entrée $input · Sortie $output · $chatTitle · $time';
  }

  @override
  String settingsDataDetailedStatsRequestModelSummary(
    int requests,
    int models,
  ) {
    return '$requests requêtes · $models modèles';
  }

  @override
  String settingsDataDetailedStatsRequestCountSummary(int requests) {
    return '$requests requêtes';
  }

  @override
  String get settingsDataDetailedStatsUnlabeledProvider =>
      'Fournisseur sans libellé';

  @override
  String get settingsDataDetailedStatsUnlabeledModel => 'Modèle sans libellé';

  @override
  String get settingsDataDetailedStatsUntitledChat => 'Conversation sans titre';

  @override
  String get settingsDataBackupSection => 'Sauvegarde et restauration';

  @override
  String get settingsDataChatHistoriesBackup => 'Données de discussion';

  @override
  String get settingsDataChatHistoriesBackupDescription =>
      'Sauvegardez toutes les discussions et tous les messages. La restauration met à jour ou crée les discussions selon leur identifiant.';

  @override
  String get settingsDataModelConfigsBackup => 'Paramètres des modèles';

  @override
  String get settingsDataModelConfigsBackupDescription =>
      'Sauvegardez tous les paramètres des modèles, y compris leurs réglages et les groupes de clés API.';

  @override
  String settingsDataBackupCount(int count) {
    return '$count éléments';
  }

  @override
  String get settingsDataExportBackupJson => 'Exporter la sauvegarde';

  @override
  String get settingsDataImportBackupJson => 'Restaurer les données';

  @override
  String settingsDataBackupImportResult(
    int newCount,
    int updatedCount,
    int skippedCount,
  ) {
    return 'Restauration terminée : $newCount nouveaux, $updatedCount mis à jour, $skippedCount ignorés.';
  }

  @override
  String settingsDataBackupImportError(String error) {
    return 'Échec de la restauration : $error';
  }

  @override
  String settingsDataBackupExportError(String error) {
    return 'Échec de l\'exportation : $error';
  }

  @override
  String get settingsDataSnapshotBackupTitle => 'Instantané complet';

  @override
  String get settingsDataExportRawSnapshot => 'Exporter l\'instantané complet';

  @override
  String get settingsDataImportRawSnapshot => 'Restaurer un instantané complet';

  @override
  String get settingsDataExportRawSnapshotDescription =>
      'Empaquete les discussions, les données des plugins, les paramètres des modèles et les fichiers locaux dans un fichier .opsnapshot.';

  @override
  String get settingsDataDatabaseBackup => 'Sauvegarde de la base de données';

  @override
  String get settingsDataDatabaseBackupDescription =>
      'Enregistre chaque jour une copie de la base de données et conserve les 10 sauvegardes automatiques les plus récentes. Les pièces jointes et les fichiers de l\'espace de travail sont stockés séparément.';

  @override
  String get settingsDataBackupScheduled => 'Sauvegarde quotidienne activée';

  @override
  String get settingsDataBackupUnavailable =>
      'Choisissez un dossier accessible pour activer les sauvegardes quotidiennes';

  @override
  String get settingsDataBackupNow => 'Sauvegarder maintenant';

  @override
  String get settingsDataBackupSaved =>
      'Sauvegarde de la base de données enregistrée.';

  @override
  String get settingsDataRecoverySection => 'Restaurer les données';

  @override
  String get settingsDataRestoreDatabaseDescription =>
      'Choisissez une sauvegarde datée de la base de données. Les données restaurées deviennent la source de référence pour la synchronisation avec les autres appareils.';

  @override
  String get settingsDataRestoreArchiveDescription =>
      'Remplace les données actuelles du runtime par un instantané complet Operit2 et redémarre l\'application.';

  @override
  String get settingsDataImportSection => 'Importer des données';

  @override
  String get settingsDataImportOperit1Description =>
      'Prévisualisez un instantané Operit1 avant de migrer les discussions, les paramètres des modèles et les fichiers.';

  @override
  String get settingsDataImportChatJson => 'Importer le JSON des discussions';

  @override
  String get settingsDataImportChatJsonDescription =>
      'Ajoute des discussions et met à jour les discussions existantes ayant un identifiant correspondant.';

  @override
  String get settingsDataImportModelJson =>
      'Importer le JSON des paramètres des modèles';

  @override
  String get settingsDataImportModelJsonDescription =>
      'Ajoute des paramètres de fournisseur de modèles et met à jour les fournisseurs existants ayant un identifiant correspondant.';

  @override
  String get settingsDataImportAction => 'Importer';

  @override
  String get settingsDataReadingSnapshot => 'Lecture de l\'instantané';

  @override
  String get settingsDataPreparingPreview => 'Préparation de l\'aperçu';

  @override
  String settingsDataOperit1ImportResult(int chatCount, int messageCount) {
    return '$chatCount discussions et $messageCount messages importés.';
  }

  @override
  String get settingsDataExportSection => 'Exporter des fichiers';

  @override
  String get settingsDataExportChatJson => 'Exporter le JSON des discussions';

  @override
  String get settingsDataExportModelJson =>
      'Exporter le JSON des paramètres des modèles';

  @override
  String settingsDataSnapshotBytes(int bytes) {
    return 'Taille de l\'instantané : $bytes octets';
  }

  @override
  String get settingsDataSnapshotImported => 'Instantané restauré.';

  @override
  String settingsDataSnapshotExportError(String error) {
    return 'Échec de l\'exportation de l\'instantané : $error';
  }

  @override
  String settingsDataSnapshotImportError(String error) {
    return 'Échec de la restauration de l\'instantané : $error';
  }

  @override
  String get settingsDataSnapshotRestoreConfirmTitle =>
      'Restaurer l\'instantané complet';

  @override
  String settingsDataSnapshotRestoreConfirmMessage(
    int formatVersion,
    int fileCount,
    String createdAt,
    int bytes,
  ) {
    return 'La restauration remplacera les données actuelles du runtime.\nVersion du format : $formatVersion\nFichiers : $fileCount\nCréé le : $createdAt\nTaille de l\'instantané : $bytes octets';
  }

  @override
  String get settingsDataSnapshotRestoreConfirmAction => 'Restaurer';

  @override
  String get settingsDataImportOperit1Snapshot => 'Importer depuis Operit1';

  @override
  String get settingsDataOperit1SnapshotImported =>
      'Instantané Operit1 importé.';

  @override
  String settingsDataOperit1SnapshotImportError(String error) {
    return 'Échec de l\'importation de l\'instantané Operit1 : $error';
  }

  @override
  String settingsDataOperit1SnapshotImportConfirmMessage(
    String fileName,
    int formatVersion,
    String chatModelId,
    int chatCount,
    int messageCount,
    int fileCount,
    int byteCount,
  ) {
    return 'Importer cet instantané Operit1 dans le runtime actuel.\nFichier : $fileName\nVersion du format : $formatVersion\nModèle de discussion : $chatModelId\nDiscussions : $chatCount ; messages : $messageCount\nFichiers de ressources : $fileCount\nTaille de l\'instantané : $byteCount octets';
  }

  @override
  String get settingsDataOperit1SnapshotImportAction => 'Importer';

  @override
  String get settingsDataAdvancedBackupOptions => 'Options avancées';

  @override
  String get settingsDataAdvancedBackupOptionsDescription =>
      'Exportation et restauration JSON par élément';

  @override
  String get settingsRuntimeNetworkControl => 'Contrôle réseau';

  @override
  String get settingsRuntimeNetworkControlDescription =>
      'Gérer les identités et autorisations des appareils, et consulter l\'activité';

  @override
  String get settingsRuntimeIdentitiesAndPermissions =>
      'Identités et autorisations';

  @override
  String get settingsRuntimeControlInitialize => 'Initialiser le contrôle';

  @override
  String get settingsRuntimeControlRoles => 'Rôles';

  @override
  String get settingsRuntimeControlIdentities => 'Identités';

  @override
  String get settingsRuntimeControlAuditAccepted => 'Appliqué';

  @override
  String get settingsRuntimeControlAuditRejected => 'Rejeté';

  @override
  String get settingsRuntimeControlAuditIssuer => 'Demandé par';

  @override
  String get settingsRuntimeControlAuditDetails => 'Détails techniques';

  @override
  String get settingsRuntimeControlAuditBootstrap =>
      'Initialiser les autorisations du Space';

  @override
  String get settingsRuntimeControlAuditAdmit => 'Admettre l\'appareil';

  @override
  String get settingsRuntimeControlAuditMerge =>
      'Fusionner les Spaces des appareils';

  @override
  String get settingsRuntimeControlAuditPolicy =>
      'Mettre à jour la politique d\'autorisations';

  @override
  String get settingsRuntimeControlAudit => 'Activité';

  @override
  String get settingsRuntimeControlAddRole => 'Nouvelle identité';

  @override
  String get settingsRuntimeControlGrantRole => 'Attribuer un rôle';

  @override
  String get settingsRuntimeControlAddIdentity => 'Ajouter une identité';

  @override
  String get settingsRuntimeControlBindDevice => 'Associer un appareil';

  @override
  String get settingsRuntimeControlRoleName => 'Nom de l\'identité';

  @override
  String get settingsRuntimeControlCapabilities =>
      'Actions autorisées pour cette identité';

  @override
  String get settingsRuntimeControlManagedDevices =>
      'Appareils gérés par ce rôle (vide = tous)';

  @override
  String get settingsRuntimeControlIdentityName => 'Nom de l\'identité';

  @override
  String get settingsRuntimeControlDeviceId => 'ID de l\'appareil';

  @override
  String get settingsRuntimeControlNoAudit =>
      'Aucune activité pour le moment. Les changements d\'identité et d\'appareil apparaîtront ici.';

  @override
  String settingsRuntimeControlAuditCount(int count) {
    return '$count entrées';
  }

  @override
  String get settingsRuntimeControlNoBindings =>
      'Aucune association d\'appareil';

  @override
  String get settingsRuntimeControlGranted => 'Accordé';

  @override
  String get settingsRuntimeControlRevoked => 'Révoqué';

  @override
  String get settingsRuntimeControlRemoveDevice => 'Retirer l\'appareil';

  @override
  String get settingsRuntimeControlDisconnectDevice => 'Isoler l\'appareil';

  @override
  String get settingsRuntimeControlRemoved => 'Retiré';

  @override
  String get settingsRuntimeControlDisconnected => 'Isolé';

  @override
  String get settingsRuntimeControlAdmitDevice => 'Lever l\'isolement';

  @override
  String get settingsRuntimeControlAllDevices => 'Tous les appareils';

  @override
  String get settingsRuntimeControlManages => 'Gère';

  @override
  String get settingsRuntimeControlLocalIdentity => 'Cet appareil';

  @override
  String get settingsRuntimeControlCurrentIdentity => 'Identité actuelle';

  @override
  String get settingsRuntimeControlCurrentCapabilities => 'Autorisations';

  @override
  String get settingsRuntimeControlManageIdentities => 'Gérer les identités';

  @override
  String get settingsRuntimeControlAssignIdentity =>
      'Définir l\'identité de l\'appareil';

  @override
  String get settingsRuntimeControlClearIdentity =>
      'Réinitialiser à l\'identité par défaut';

  @override
  String get settingsRuntimeControlDeviceAdmitted =>
      'Isolement levé ; en attente de reconnexion de l\'appareil';

  @override
  String get settingsRuntimeControlIdentityResetDone =>
      'Identité réinitialisée à l\'utilisateur par défaut';

  @override
  String get settingsRuntimeControlIdentityDefinitions =>
      'Autorisations d\'identité';

  @override
  String settingsRuntimeControlIdentityCount(int count) {
    return '$count identités';
  }

  @override
  String get settingsRuntimeControlIdentityAssignment =>
      'Attribution d\'identité';

  @override
  String get settingsRuntimeControlNoIdentity => 'Aucune identité attribuée';

  @override
  String get settingsRuntimeControlOnline => 'En ligne';

  @override
  String get settingsRuntimeControlOffline => 'Hors ligne';

  @override
  String get settingsRuntimeControlCapabilityAll => 'Tout';

  @override
  String get settingsRuntimeControlCapabilityAuditRead =>
      'Consulter l\'historique d\'audit';

  @override
  String get settingsRuntimeControlCapabilityNetworkRelay =>
      'Relayer le trafic réseau';

  @override
  String get settingsRuntimeControlCapabilityStorageProvide =>
      'Stocker des répliques de données partagées';

  @override
  String get settingsRuntimeControlCapabilityRuntimeExecute =>
      'Héberger l\'exécution du Core';

  @override
  String get settingsRuntimeControlCapabilityViewDevices =>
      'Voir les appareils et les identités actuelles';

  @override
  String get settingsRuntimeControlCapabilityApproval =>
      'Approuver les autorisations';

  @override
  String chatRoutePermissionDenied(Object targetNodeId, Object capability) {
    return 'Autorisation refusée : l\'appareil cible $targetNodeId ne dispose pas de la capacité $capability.';
  }

  @override
  String get settingsRuntimeControlRoleAdministrator => 'Administrateur';

  @override
  String get settingsRuntimeControlRoleRelay => 'Nœud relais';

  @override
  String get settingsRuntimeControlRoleEdge => 'Nœud périphérique';

  @override
  String get settingsRuntimeControlRoleUser => 'Utilisateur';

  @override
  String get pluginLoadingTitle => 'Chargement';

  @override
  String get pluginLoadingSkip => 'Ignorer';

  @override
  String get pluginLoadingCollapse => 'Réduire';

  @override
  String pluginLoadingStatus(int started, int total) {
    return 'Démarrés : $started / $total';
  }

  @override
  String pluginLoadingCurrentTask(String task) {
    return 'Chargement de $task';
  }

  @override
  String get pluginLoadingSuccess => 'Chargé';

  @override
  String get pluginLoadingFailed => 'Échec';

  @override
  String get pluginLoadingWaiting => 'En attente';

  @override
  String get pluginLoadingKindPackage => 'Paquet';

  @override
  String get pluginLoadingKindPlugin => 'Plugin';

  @override
  String get pluginLoadingKindMcp => 'MCP';

  @override
  String get pluginLoadingCompleteSuccess =>
      'Tous les paquets et plugins sont chargés';

  @override
  String get pluginLoadingCompleteWithFailures =>
      'Échec du chargement de certains paquets ou plugins';

  @override
  String get pluginLoadingNoLogs => 'Aucun journal';

  @override
  String get chatRouteLocal => 'Local';

  @override
  String chatRouteRemote(String platform) {
    return '$platform distant';
  }

  @override
  String chatRouteRemoteOffline(String platform) {
    return '$platform distant hors ligne';
  }

  @override
  String get chatRouteLoading => 'Chargement de l\'appareil d\'exécution';

  @override
  String get chatRouteUnknown => 'État de l\'appareil d\'exécution inconnu';

  @override
  String get chatRouteClaimHostPrompt =>
      'L\'appareil hôte de cette discussion est introuvable. Basculer vers cet appareil ?';

  @override
  String get chatRouteClaimHostYes => 'Oui';

  @override
  String get chatRouteClaimHostNo => 'Non';

  @override
  String edgePairingCodeTitle(String deviceName) {
    return 'Saisissez le code d\'appairage pour $deviceName';
  }

  @override
  String get edgePairingCodeHint =>
      'Lisez le code à six chiffres affiché par l\'appareil.';

  @override
  String get chatMessageResponse => 'Réponse';

  @override
  String get chatMessagePrompt => 'Prompt';

  @override
  String chatMessagePromptBy(String sender) {
    return 'Prompt de $sender';
  }

  @override
  String chatMessageTokensTooltip(String input, String output) {
    return 'Tokens : ↑$input · ↓$output';
  }

  @override
  String chatMessageTokensCachedTooltip(
    String input,
    String cached,
    String output,
  ) {
    return 'Tokens : ↑$input (en cache : $cached) · ↓$output';
  }

  @override
  String chatMessageTimingTooltip(String wait, String output) {
    return 'Durée : $wait d\'attente · $output de sortie';
  }

  @override
  String chatMessageTimeTooltip(String time) {
    return 'Heure : $time';
  }

  @override
  String get chatMessageInfoDialogTitle => 'Détails du message';

  @override
  String get chatMessageDeleteConfirmTitle => 'Confirmer la suppression';

  @override
  String get chatMessageDeleteConfirmMessage => 'Supprimer ce message ?';

  @override
  String chatMessageSender(String sender) {
    return 'Expéditeur : $sender';
  }

  @override
  String chatMessageTimestamp(String timestamp) {
    return 'Horodatage : $timestamp';
  }

  @override
  String chatMessageRole(String roleName) {
    return 'Répondeur : $roleName';
  }

  @override
  String chatMessageModel(String modelName) {
    return 'Modèle : $modelName';
  }

  @override
  String chatMessageProvider(String provider) {
    return 'Fournisseur : $provider';
  }

  @override
  String chatMessageTokensInput(String count) {
    return 'Tokens d\'entrée : $count';
  }

  @override
  String chatMessageTokensCached(String count) {
    return 'Tokens d\'entrée en cache : $count';
  }

  @override
  String chatMessageTokensOutput(String count) {
    return 'Tokens de sortie : $count';
  }

  @override
  String chatMessageWaitDuration(String duration) {
    return 'Temps d\'attente : $duration';
  }

  @override
  String chatMessageOutputDuration(String duration) {
    return 'Durée de sortie : $duration';
  }

  @override
  String chatMessageTokenSpeed(String speed) {
    return 'Vitesse des tokens : $speed';
  }

  @override
  String chatMessageCacheShort(String count) {
    return 'cache $count';
  }

  @override
  String chatMessageCacheHitRate(String rate) {
    return 'Taux de succès du cache : $rate';
  }

  @override
  String get settingsPeerPortModeHelp =>
      'Par défaut, le système attribue un port disponible et la découverte LAN l\'annonce. Le mode fixe utilise l\'adresse et le port d\'écoute saisis et signale une erreur si le port est occupé.';

  @override
  String get settingsPeerPortMode => 'Attribution de port';

  @override
  String get settingsPeerPortModeFixed => 'Adresse et port d\'écoute fixes';

  @override
  String get settingsPeerPortModeAutomatic => 'Port automatique';

  @override
  String get settingsPeerNetworkDiscoveryHelp =>
      'Pour la découverte à travers des réseaux routés, configurez un annuaire partagé et une clé de groupe. Pour héberger l\'annuaire, laissez l\'adresse vide et définissez une clé de groupe.';

  @override
  String get settingsPeerDiscoveryDirectory =>
      'Annuaire de découverte (facultatif)';

  @override
  String get settingsPeerDiscoveryGroup => 'Clé de groupe de découverte';

  @override
  String get settingsPeerDiscoveryGenerateKey =>
      'Générer une nouvelle clé de groupe';

  @override
  String get settingsPeerDiscoveryCopyKey => 'Copier la clé de groupe';

  @override
  String get settingsPeerDiscoveryPort => 'Port d\'écoute de l\'annuaire';

  @override
  String get settingsPeerDiscoveryPortHelp =>
      'Utilisé uniquement sur l\'hôte de l\'annuaire. Les autres appareils utilisent le port indiqué dans l\'URL de l\'annuaire.';

  @override
  String get settingsPeerNetworkDiscoveryInvalid =>
      'Vérifiez les paramètres de découverte : utilisez une origine HTTP(S) sans chemin, une clé partagée de 16 à 128 octets et un port non nul pour héberger l\'annuaire.';

  @override
  String get settingsPeerAdvanced => 'Options avancées';

  @override
  String get settingsPeerDiscoveryUnavailable =>
      'Cet hôte ne peut pas annoncer d\'écouteurs sur le réseau local. Les connexions directes restent disponibles pour les transports pris en charge.';

  @override
  String settingsPeerTransportUnavailable(String transport) {
    return 'L\'écoute $transport n\'est pas prise en charge par cet hôte.';
  }

  @override
  String get settingsPeerTransportHelp =>
      'Transports d\'écoute (sélection multiple). HTTP, WebSocket et TCP partagent un même port d\'écoute et peuvent être combinés librement. Bluetooth nécessite une prise en charge de la plateforme. Désélectionnez tout et désactivez la découverte pour arrêter l\'écoute. La modification des paramètres interrompt les connexions existantes. Désactiver la découverte seule n\'interrompt pas les connexions directes.';

  @override
  String get settingsPeerBindAddress => 'Adresse IP d\'écoute';

  @override
  String get settingsPeerBindAddressHelp =>
      'Exemple : 0.0.0.0 pour un accès LAN ; les adresses IPv6 sont également prises en charge.';

  @override
  String get settingsPeerBindPort => 'Port d\'écoute';

  @override
  String get settingsPeerBindPortHelp => 'Port utilisé en mode d\'écoute fixe.';

  @override
  String get settingsPeerBindPortAutomaticHelp =>
      'Le système attribue un port disponible. Le port réel s\'affiche après le démarrage.';

  @override
  String get settingsPeerDiscoveryNeedsTransport =>
      'Sélectionnez au moins un transport avant d\'activer la découverte.';

  @override
  String get settingsPeerAddressRequired =>
      'Saisissez une adresse IP d\'écoute.';

  @override
  String get settingsPeerPortRequired =>
      'Saisissez un port compris entre 1 et 65535.';

  @override
  String get settingsPeerApplied => 'Paramètres d\'écoute appliqués.';

  @override
  String settingsPeerRestoreFailed(String error) {
    return 'Échec de la restauration des écouteurs précédents : $error';
  }

  @override
  String get settingsPeerCodeInstructions =>
      'Saisissez ce code sur l\'autre appareil pour terminer l\'appairage. Il expire dans cinq minutes. Ne le partagez pas avec des inconnus.';

  @override
  String get settingsPeerSixDigitCode =>
      'Saisissez le code d\'appairage à six chiffres affiché sur l\'appareil récepteur.';

  @override
  String get spaceJoinRequests => 'Demandes d\'adhésion au Space';

  @override
  String spaceJoinReviewer(String name) {
    return 'Réviseur : $name';
  }

  @override
  String get spaceJoinWaitingReviewer =>
      'En attente de la connexion d\'un réviseur autorisé';

  @override
  String get spaceJoinPending => 'En attente d\'approbation';

  @override
  String get spaceJoinApproving => 'Décision en cours de traitement';

  @override
  String get spaceJoinApproved => 'Approuvé · finalisation de l\'adhésion';

  @override
  String get spaceJoinJoined => 'Rejoint';

  @override
  String get spaceJoinRejected => 'Refusée';

  @override
  String get spaceJoinCancelled => 'Annulée';

  @override
  String get spaceJoinExpired => 'Expirée';

  @override
  String get spaceJoinSubmit => 'Envoyer la demande';

  @override
  String get spaceJoinApprovalTitle => 'Demande d\'accès au Space';

  @override
  String spaceJoinApprovalDescription(String name, String space) {
    return '$name souhaite rejoindre « $space ». Aucune donnée du Space n\'est partagée avant approbation.';
  }

  @override
  String get spaceJoinApprove => 'Approuver';

  @override
  String get spaceJoinReject => 'Refuser';

  @override
  String get spaceJoinRetry => 'Envoyer à nouveau';

  @override
  String get spaceJoinKeepWaiting => 'Continuer d\'attendre en arrière-plan';

  @override
  String get spaceJoinCancel => 'Annuler la demande';

  @override
  String get spaceJoinCancelFailed =>
      'Impossible de confirmer l\'annulation. Vérifiez la connexion ou réessayez plus tard.';

  @override
  String get spaceJoinRefreshingFailed =>
      'Impossible de joindre l\'appareil pour le moment. La demande est enregistrée et sera réessayée.';

  @override
  String get spaceJoinAssignmentChanged =>
      'Cette demande a été transférée ou déjà traitée.';

  @override
  String get spaceJoinProgressTitle => 'Demande d\'accès au Space';

  @override
  String get spaceJoinSending => 'Envoi de la demande…';

  @override
  String get spaceJoinSubmitFailed =>
      'Impossible d\'envoyer la demande. Vérifiez la connexion et réessayez.';

  @override
  String get spaceJoinNoDataYet =>
      'Votre Space actuel reste inchangé jusqu\'à l\'approbation. Vous pouvez fermer cette fenêtre et continuer d\'attendre en arrière-plan.';

  @override
  String get deviceSpaceAddDevice => 'Ajouter un appareil';

  @override
  String get deviceSpaceDevices => 'Appareils';

  @override
  String get deviceSpaceMemberNotPaired => 'Membre du Space · non appairé';

  @override
  String get deviceSpaceMemberPaired => 'Membre du Space · appairé';

  @override
  String get deviceSpacePairedOnly => 'Appairé';

  @override
  String get deviceSpaceDirectLink => 'Direct';

  @override
  String deviceSpaceRelayHops(int hops) {
    return 'Relayé · $hops sauts';
  }

  @override
  String get deviceSpaceRelayPath => 'Chemin';

  @override
  String get deviceSpaceLocalDevice => 'Cet appareil';

  @override
  String get deviceSpaceManageDevice => 'Gérer l\'appareil';

  @override
  String get deviceSpaceConnectDirect => 'Connexion directe';

  @override
  String get deviceSpaceConnectDirectHint =>
      'Cet appareil est déjà membre du Space. L\'appairage ajoute un lien direct sans rejoindre à nouveau ; un code à six chiffres s\'affichera sur l\'autre appareil pour que vous puissiez le saisir.';

  @override
  String get deviceSpaceUpdateEndpoint => 'Mettre à jour l\'adresse directe';

  @override
  String get deviceSpaceUpdateEndpointLabel => 'Adresse directe';

  @override
  String get deviceSpaceUpdateEndpointHint =>
      'ex. ws://host:port/link ou host:port — utilisez le port d\'écoute actuel du pair';

  @override
  String get deviceSpaceUpdateEndpointSaved => 'Adresse directe mise à jour';

  @override
  String deviceSpaceDirectLinkEstablished(String device) {
    return 'Lien direct établi : $device';
  }

  @override
  String get deviceSpaceUnpair => 'Dissocier l\'appareil';

  @override
  String get deviceSpaceMore => 'Plus d\'actions sur l\'appareil';

  @override
  String get deviceSpaceConnectionSettings => 'Paramètres de connexion';

  @override
  String get deviceSpaceViewRequest => 'Voir la demande';

  @override
  String get devicePickerNearby => 'Appareils à proximité';

  @override
  String devicePickerDiscoveryFailed(String error) {
    return 'Échec de la découverte d\'appareils à proximité : $error';
  }

  @override
  String get devicePickerEmpty => 'Aucun appareil à proximité trouvé';

  @override
  String get devicePickerHint =>
      'Connectez les deux appareils au même réseau et activez la découverte sur l\'autre appareil.';

  @override
  String get devicePickerManual => 'Se connecter via une adresse';

  @override
  String get deviceSpaceMyRequests => 'Mes demandes';

  @override
  String get deviceSpaceMyReviews => 'En attente de ma validation';

  @override
  String get deviceSpaceNoRequests => 'Aucune demande';

  @override
  String get workspaceUndo => 'Annuler';

  @override
  String get workspaceRedo => 'Rétablir';

  @override
  String workspaceInsertSymbol(String symbol) {
    return 'Insérer $symbol';
  }

  @override
  String get settingsAppearanceThemeConfig => 'Configuration du thème';

  @override
  String get settingsAppearanceThemeConfigCustom => 'Apparence libre';

  @override
  String get settingsAppearanceThemeConfigCreate =>
      'Nouvelle configuration de thème';

  @override
  String get settingsAppearanceThemeConfigRename =>
      'Renommer la configuration de thème';

  @override
  String get settingsAppearanceThemeConfigDelete =>
      'Supprimer une configuration de thème inactive';

  @override
  String get settingsAppearanceThemeConfigName => 'Nom du thème';

  @override
  String get settingsAppearanceThemeConfigNameRequired =>
      'Saisissez un nom de thème';

  @override
  String get settingsAppearanceThemeConfigCustomHint =>
      'Créez un thème pour enregistrer votre apparence. Touchez une carte pour basculer ; choisissez Modifier pour personnaliser.';

  @override
  String get settingsAppearanceThemeConfigEditingHint =>
      'Touchez une carte pour changer de thème. Les raccourcis de mode ne modifient que le thème actuel. Les modifications sont enregistrées automatiquement.';

  @override
  String settingsAppearanceThemeConfigFailed(String detail) {
    return 'Échec de l\'opération sur le thème : $detail';
  }

  @override
  String get settingsRuntimeControlEditRole => 'Modifier le rôle';

  @override
  String get settingsRuntimeControlDeleteRole => 'Supprimer le rôle';

  @override
  String settingsRuntimeControlDeleteRoleTitle(String roleName) {
    return 'Supprimer le rôle $roleName ?';
  }

  @override
  String get settingsRuntimeControlDeleteRoleMessage =>
      'Les appareils disposant de ce rôle reviennent à l\'identité utilisateur par défaut.';

  @override
  String get settingsAppearanceThemeConfigSource => 'Créer à partir de';

  @override
  String get settingsAppearanceThemeConfigFromDefaults =>
      'Apparence par défaut';

  @override
  String get settingsAppearanceThemeConfigFromCurrent => 'Apparence actuelle';

  @override
  String settingsAppearanceThemeConfigInherit(String name) {
    return 'Hériter : $name';
  }

  @override
  String settingsAppearanceThemeConfigFromTemplate(String name) {
    return 'Modèle : $name';
  }

  @override
  String get settingsAppearanceThemeConfigDefaultsHint =>
      'Partir des valeurs par défaut de l\'application';

  @override
  String get settingsAppearanceThemeConfigCurrentHint =>
      'Copier votre apparence actuelle';

  @override
  String get settingsAppearanceThemeConfigSavedSources =>
      'Hériter d\'une configuration enregistrée';

  @override
  String get settingsAppearanceThemeConfigPluginSources =>
      'Utiliser un modèle de plugin';

  @override
  String get settingsAppearanceThemeConfigSourceHint =>
      'Les modèles proviennent des plugins installés. Les nouvelles configurations peuvent être modifiées indépendamment et utilisent la synchronisation d\'apparence existante.';

  @override
  String get settingsAppearanceLoadFailed =>
      'Impossible de charger l\'apparence';

  @override
  String get settingsAppearanceRepairHint =>
      'Votre apparence enregistrée n\'a pas été remplacée. Réessayez après avoir réparé ses ressources ou réinitialisez l\'apparence manuellement. La réinitialisation conserve votre avatar et vos configurations enregistrées, et sélectionne l\'apparence personnalisée.';

  @override
  String settingsAppearanceAssetBackgroundFailed(String reference) {
    return 'Impossible de charger l\'image d\'arrière-plan. Utilisation de l\'arrière-plan par défaut ($reference).';
  }

  @override
  String settingsAppearanceAssetVideoFailed(String reference) {
    return 'Impossible de lire la vidéo d\'arrière-plan. Utilisation de l\'arrière-plan par défaut ($reference).';
  }

  @override
  String settingsAppearanceAssetBubbleFailed(String reference) {
    return 'Impossible de charger l\'image de bulle. Utilisation de la couleur de bulle par défaut ($reference).';
  }

  @override
  String settingsAppearanceAssetPreviousRetained(String reference) {
    return 'Impossible de charger cette ressource de thème. Conservation de l\'image ou du média précédent ($reference) ; vos autres paramètres de thème restent appliqués.';
  }

  @override
  String get settingsAppearanceAssetRetry => 'Réessayer';

  @override
  String get settingsAppearanceAssetReimportHint =>
      'Si réessayer ne suffit pas, sélectionnez et importez à nouveau une image lisible ou une vidéo compatible.';

  @override
  String settingsAppearanceAssetImportFailed(String reason) {
    return 'La ressource d\'arrière-plan n\'a pas été appliquée : $reason. Veuillez choisir une image lisible ou une vidéo compatible.';
  }

  @override
  String get settingsAppearanceRetry => 'Réessayer';

  @override
  String get settingsAppearanceResetAfterError => 'Réinitialiser l\'apparence';

  @override
  String get backgroundWorkIdle => 'Aucune tâche en cours';

  @override
  String get backgroundWorkRunningCount => 'tâches en cours';

  @override
  String get backgroundWorkWaiting => 'En attente de votre confirmation';

  @override
  String get backgroundWorkWaitingCount => 'en attente de confirmation';

  @override
  String get backgroundWorkCompleted => 'Terminée';

  @override
  String get backgroundWorkFailed => 'Échec — ouvrir pour voir les détails';

  @override
  String get backgroundWorkCancelled => 'Arrêtée';

  @override
  String get backgroundWorkProcessing => 'Préparation';

  @override
  String get backgroundWorkConnecting => 'Connexion';

  @override
  String get backgroundWorkReceiving => 'Rédaction d\'une réponse';

  @override
  String get backgroundWorkTool => 'Exécution d\'un outil';

  @override
  String get backgroundWorkToolResult => 'Traitement des résultats de l\'outil';

  @override
  String get backgroundWorkSummarizing => 'Organisation du contexte';

  @override
  String get backgroundWorkPlan => 'Exécution d\'un plan';

  @override
  String get backgroundWorkOpen => 'Voir la conversation';

  @override
  String get backgroundWorkReview => 'Examiner';

  @override
  String get backgroundWorkStop => 'Arrêter';

  @override
  String get localModelsDeviceOnly => 'Stocké sur cet appareil';

  @override
  String get localModelsDeviceOnlyDescription =>
      'Les poids, la progression des téléchargements et l\'historique d\'installation restent sur cet appareil et sont exclus de la synchronisation Space. Les discussions peuvent rediriger l\'inférence vers l\'appareil hébergeant le modèle.';

  @override
  String get localModelsRunChat => 'Exécuter et configurer';

  @override
  String get localModelsUseForChat => 'Utiliser pour le chat';

  @override
  String get localModelsChatActivated => 'Sélectionné comme modèle de chat';

  @override
  String get localModelsPrompt => 'Saisissez un prompt';

  @override
  String get localModelsGenerate => 'Générer';

  @override
  String get localModelsGenerating => 'Génération…';

  @override
  String get localModelsThreads => 'Threads CPU';

  @override
  String get localModelsContextSize => 'Taille du contexte (tokens)';

  @override
  String get localModelsBatchSize => 'Taille du lot';

  @override
  String get localModelsUnload => 'Décharger le modèle';

  @override
  String get localModelsCpuBackend => 'Inférence CPU sur l\'appareil';

  @override
  String get localModelsLlmDescription =>
      'Téléchargez un modèle GGUF pour llama.cpp ou un pack de modèle MNN. Après l\'installation, testez-le ici ou sélectionnez-le pour le chat.';

  @override
  String get localModelsInvalidSettings =>
      'Vérifiez le nombre de threads, la taille du contexte et la taille du lot.';

  @override
  String get localModelsFilterAll => 'Tous les modèles';

  @override
  String get localModelsFilterInstalled => 'Installés uniquement';

  @override
  String get localModelsSearchCatalog =>
      'Rechercher des modèles, tags ou moteurs';

  @override
  String get deviceSpaceSyncPeer => 'un appareil Space';

  @override
  String deviceSpaceSyncActive(String device) {
    return 'Synchronisation avec $device';
  }

  @override
  String deviceSpaceSyncCompleted(String device) {
    return 'Synchronisé avec $device';
  }

  @override
  String get deviceSpaceSyncChecking => 'Recherche de modifications';

  @override
  String get deviceSpaceSyncRecords => 'Échange d\'enregistrements';

  @override
  String deviceSpaceSyncRecordProgress(int completed, int total, int percent) {
    return '$completed / $total enregistrements · $percent%';
  }

  @override
  String deviceSpaceSyncPendingCount(int count) {
    return '$count enregistrements en attente d\'application';
  }

  @override
  String deviceSpaceSyncCounting(int count) {
    return 'Calcul du total · $count enregistrements traités';
  }

  @override
  String get deviceSpaceSyncFiles => 'Transfert et vérification des fichiers';

  @override
  String get deviceSpaceSyncRetrying =>
      'Synchronisation interrompue. Nouvelle tentative automatique.';

  @override
  String get deviceSpaceSyncPending =>
      'Certaines données sont encore en attente.';

  @override
  String deviceSpaceSyncCurrentFile(String done, String total) {
    return 'Fichier actuel : $done / $total';
  }

  @override
  String deviceSpaceSyncTransferred(int count, String bytes) {
    return '$count enregistrements échangés · $bytes transférés';
  }

  @override
  String get deviceSpaceSyncBackground =>
      'Vous pouvez continuer à utiliser l\'application pendant la synchronisation.';

  @override
  String get runtimeActivityTitle => 'Tâches en arrière-plan';

  @override
  String get runtimeActivitySyncing => 'Synchronisation';

  @override
  String get runtimeActivitySyncCompleted => 'Synchronisation terminée';

  @override
  String get runtimeActivityWaiting => 'En attente de nouvelle tentative';

  @override
  String get runtimeActivityExpand => 'Afficher les tâches en arrière-plan';

  @override
  String get runtimeActivityLoadingPackages => 'Chargement des extensions';

  @override
  String get peerQrTitle => 'Connexion par QR';

  @override
  String get peerQrScan => 'Scanner un code';

  @override
  String get peerQrShow => 'Mon code';

  @override
  String get peerQrShowHint =>
      'Scannez ce code sur un autre appareil pour vous connecter à celui-ci.';

  @override
  String get peerQrScanHint =>
      'Scannez le code de connexion Operit sur l\'autre appareil.';

  @override
  String get peerQrTrustHint =>
      'Ce code QR est valide pendant 5 minutes. Ne le partagez qu\'avec des appareils de confiance. La confirmation permet de rejoindre directement ; cet appareil approuve automatiquement s\'il dispose des droits d\'approbation administrateur.';

  @override
  String get peerQrConfirmJoin => 'Confirmer l\'adhésion';

  @override
  String get peerQrConfirmJoinHint =>
      'Rejoindre le Space de cet appareil ? Confirmez pour vous connecter et envoyer la demande d\'adhésion sans saisir de code d\'appairage.';

  @override
  String get peerQrListenerRequired =>
      'Activez la réception des connexions dans les paramètres de connexion et connectez-vous d\'abord à un réseau.';

  @override
  String get peerQrCopy => 'Copier les infos de connexion';

  @override
  String get peerQrCopied => 'Copié';

  @override
  String get peerQrImport => 'Importer une image QR';

  @override
  String get peerQrPaste => 'Ou coller les infos de connexion';

  @override
  String get peerQrRead => 'Lire les infos de connexion';

  @override
  String get peerQrWrongDevice =>
      'Ce code appartient à un autre appareil. Scannez le code de l\'appareil sélectionné.';

  @override
  String get peerPairTimeoutHint =>
      'Délai de connexion dépassé : impossible de joindre l\'autre appareil. Une cause fréquente est le pare-feu de l\'autre appareil qui bloque les connexions entrantes. Essayez d\'initier la connexion depuis l\'autre appareil ou autorisez Operit dans son pare-feu (en particulier sur les réseaux publics).';

  @override
  String get peerQrImageInvalid =>
      'Aucun code QR n\'a pu être lu. Choisissez une image QR nette de moins de 16 Mo.';

  @override
  String get peerQrCameraUnavailable =>
      'Le scan par caméra est indisponible. Importez une image QR ou collez les infos de connexion à la place.';

  @override
  String get peerQrCameraPermission =>
      'L\'accès à la caméra est requis. Autorisez-le dans les paramètres système ou importez une image QR.';

  @override
  String get peerQrDevice => 'Connecter l\'appareil';

  @override
  String get peerQrScanAgain => 'Scanner un autre code';

  @override
  String get peerQrMethodsHint =>
      'Appareil introuvable ? Connectez-vous avec un code QR.';

  @override
  String get toolParametersLabel => 'Paramètres';

  @override
  String get toolResultLabel => 'Résultat';

  @override
  String toolGroupThinkingCount(int count) {
    return '$count réflexion';
  }

  @override
  String toolGroupCallCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count appels',
      one: '$count appel',
    );
    return '$_temp0';
  }

  @override
  String toolGroupSearchCount(int count) {
    return '$count recherche';
  }

  @override
  String toolEditedFileCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count fichiers modifiés',
      one: '$count fichier modifié',
    );
    return '$_temp0';
  }

  @override
  String get toolViewChanges => 'Voir les modifications';

  @override
  String toolViewFileDiff(String fileName) {
    return 'Voir le diff de $fileName';
  }

  @override
  String get toolGroupCompleted => 'Terminé';

  @override
  String get toolGroupExecuting => 'En cours d\'exécution';

  @override
  String get settingsDataTimelineTitle =>
      'Utilisation des modèles dans le temps';

  @override
  String get settingsDataTimelineHint =>
      'Une courbe par modèle. Pincez ou faites défiler pour zoomer, glissez pour déplacer et touchez pour inspecter une tranche horaire.';

  @override
  String settingsDataTimelineHours(int hours) {
    return '$hours h';
  }

  @override
  String settingsDataTimelineBucket(int minutes) {
    return 'Tranches de $minutes min';
  }

  @override
  String settingsDataTimelineSecondsBucket(int seconds) {
    return 'Tranches de $seconds s';
  }

  @override
  String get settingsDataTimelineEarlier => 'Plus tôt';

  @override
  String get settingsDataTimelineLater => 'Plus tard';

  @override
  String get settingsDataTimelineRecent => '3 dernières h';

  @override
  String get settingsDataTimelineRange => 'Plage horaire visible';

  @override
  String get settingsDataTimelineSelectModel =>
      'Sélectionnez un modèle ci-dessous pour afficher son utilisation';

  @override
  String get settingsDataTimelineSpend => 'Dépenses (estimées)';

  @override
  String get settingsDataTimelineCacheRate => 'Taux de succès du cache';

  @override
  String get settingsDataTimelineSpendHint =>
      'Estimé selon les tarifs actuels des modèles. Partage la plage horaire ci-dessus.';

  @override
  String get settingsDataTimelineCacheRateHint =>
      'Entrées en cache / total des entrées par tranche horaire. Les périodes d\'inactivité n\'ont pas de taux.';

  @override
  String get settingsDataTimelineNoPrices =>
      'Aucun tarif de modèle correspondant';

  @override
  String get settingsDataTimelinePricesFailed =>
      'Impossible de charger les tarifs des modèles. Actualisez pour réessayer.';

  @override
  String settingsDataTimelineUnpriced(int count) {
    return '$count requêtes au coût inconnu';
  }

  @override
  String get settingsDataTimelineKnownSpend => 'Dépenses connues (estimées)';

  @override
  String get settingsDataDetailedStatsRangeLastDay => 'Dernières 24 heures';

  @override
  String get settingsDataTimelineAverageInput => 'Entrée moyenne par requête';

  @override
  String get settingsDataTimelineAverageInputHint =>
      'Tokens d\'entrée / requêtes par tranche, pour repérer l\'augmentation de la taille du contexte. Les périodes d\'inactivité apparaissent comme des vides.';

  @override
  String get settingsDataTimelineCacheSavings =>
      'Économies de cache (estimées)';

  @override
  String get settingsDataTimelineCacheSavingsHint =>
      'Aux tarifs actuels, coût des entrées en cache comparé aux entrées standard. La facturation à la requête économise 0 ; les valeurs négatives indiquent un surcoût de cache.';

  @override
  String settingsDataTimelineTokensPerRequest(String tokens) {
    return '$tokens tokens / requête';
  }

  @override
  String settingsDataTimelineUnknownSavings(int count) {
    return 'Économies inconnues pour $count requêtes';
  }

  @override
  String get contextStatsSummaryLoadFailed =>
      'Impossible de charger les statistiques de contexte';

  @override
  String get contextStatsDevicesLoadFailed =>
      'Impossible de charger les informations de l\'appareil';

  @override
  String get settingsAppearanceThemeLibrary => 'Vos thèmes';

  @override
  String get settingsAppearanceThemeLibraryHint =>
      'Choisissez un style. Faites-le vôtre.';

  @override
  String get settingsAppearanceThemeActive => 'Actif';

  @override
  String get settingsAppearanceThemeApply => 'Utiliser ce thème';

  @override
  String get settingsAppearanceThemeMore => 'Plus d\'actions sur le thème';

  @override
  String get settingsAppearanceThemeDuplicate => 'Créer une copie';

  @override
  String get settingsAppearanceThemeUnboundHint =>
      'Conservez l\'apparence actuelle et modifiez-la sans la lier à un thème enregistré.';

  @override
  String get settingsAppearanceThemeDeleteTitle => 'Supprimer le thème';

  @override
  String settingsAppearanceThemeDeleteConfirm(String name) {
    return 'Supprimer « $name » ? Cette action est irréversible.';
  }

  @override
  String get settingsAppearancePreviewCollapse => 'Réduire';

  @override
  String get settingsAppearancePreviewExpand => 'Développer';

  @override
  String get settingsAppearanceThemeNew => 'Nouveau thème';

  @override
  String get settingsAppearanceDeviceFonts => 'Polices de l\'appareil';

  @override
  String get settingsAppearanceChooseDeviceFont =>
      'Choisir une police de l\'appareil';

  @override
  String get settingsAppearanceNoDeviceFonts =>
      'Aucune police d\'appareil correspondante';

  @override
  String get settingsAppearanceDeviceFontPreview =>
      'Voix ambiguë d\'un cœur · 0123456789\n字体预览：你好，世界';

  @override
  String get workspaceFileRename => 'Renommer';

  @override
  String get workspaceFileCopy => 'Copier';

  @override
  String get workspaceFileCut => 'Couper';

  @override
  String get workspaceFilePaste => 'Coller';

  @override
  String get workspaceFileName => 'Nom';

  @override
  String get workspaceFileInvalidName =>
      'Saisissez un nom valide sans séparateurs de chemin.';

  @override
  String get workspaceDeleteFolderTitle => 'Supprimer le dossier ?';

  @override
  String workspaceDeleteFolderMessage(String name) {
    return 'Supprimer « $name » et tout son contenu ? Cette action est irréversible.';
  }

  @override
  String workspaceFileOperationFailed(String error) {
    return 'Échec de l\'opération sur le fichier : $error';
  }

  @override
  String workspaceFileCopyName(String name, int index) {
    return '$name (copie $index)';
  }

  @override
  String get workspaceFileSaving =>
      'Ce fichier est en cours d\'enregistrement. Veuillez réessayer une fois l\'enregistrement terminé.';

  @override
  String get chatBackupChooseTitle => 'Choisissez votre dossier de sauvegarde';

  @override
  String get chatBackupTitle => 'Sauvegarde quotidienne du chat';

  @override
  String get chatBackupDescription =>
      'Copie chaque jour la base de données SQLite complète lorsque l\'application est en cours d\'exécution et conserve les 10 dernières sauvegardes automatiques. Inclut toutes les données stockées dans cette base. Les pièces jointes externes, les espaces de travail et les fichiers de configuration séparés ne sont pas copiés.';

  @override
  String get chatRestoreTitle =>
      'Restaurer depuis une sauvegarde de base de données';

  @override
  String get chatRestoreDescription =>
      'Ouvrez le dossier contenant vos sauvegardes et choisissez la date à restaurer.';

  @override
  String get chatRestoreChooseFolder => 'Choisir la source de sauvegarde';

  @override
  String get chatRestoreRefresh => 'Actualiser la liste';

  @override
  String get chatRestoreSelected => 'Restaurer la sauvegarde sélectionnée';

  @override
  String get chatRestoreAction => 'Restaurer';

  @override
  String get chatRestoreCancel => 'Annuler';

  @override
  String get chatRestoreEmpty =>
      'Ce dossier ne contient aucune sauvegarde de base de données.';

  @override
  String get chatRestoreConfirmTitle =>
      'Restaurer cette sauvegarde de base de données ?';

  @override
  String chatRestoreConfirmDescription(String name) {
    return 'Remplacer le fichier de base de données actuel par $name. Cela remplace toutes les données stockées dans cette base. Arrêtez d\'abord les conversations en cours. La restauration établit une nouvelle référence de synchronisation qui remplace les données plus anciennes sur les autres appareils.';
  }

  @override
  String get chatRestoreSuccess =>
      'Base de données restaurée. La prochaine synchronisation utilisera cette copie comme source de référence.';

  @override
  String get chatRestoreOnboardingTitle =>
      'Restaurer depuis une sauvegarde précédente';

  @override
  String get chatRestoreOnboardingDescription =>
      'Récupérez les discussions enregistrées dans un dossier externe avant une réinstallation ou désinstallation';

  @override
  String get chatBackupExternalDescription =>
      'Choisissez un dossier hors de l\'application afin que les sauvegardes soient conservées après désinstallation. Sur iPhone et iPad, choisissez iCloud Drive ou un autre fournisseur de fichiers externe.';

  @override
  String get chatBackupChoose => 'Choisir le dossier de sauvegarde';

  @override
  String get chatBackupLater => 'Configurer plus tard';

  @override
  String get chatBackupChange => 'Changer le dossier de sauvegarde';

  @override
  String get chatBackupMissing =>
      'Le dossier de sauvegarde n\'est pas configuré, n\'existe plus ou est inaccessible. Sélectionnez un dossier pour activer les sauvegardes quotidiennes du chat.';

  @override
  String get chatBackupDirectoryUnavailable =>
      'Le dossier sélectionné ne peut pas être utilisé. Créez un nouveau dossier dans un emplacement accessible, comme Documents ou Téléchargements, puis sélectionnez-le.';

  @override
  String get chatBackupReady =>
      'Le dossier de sauvegarde est prêt. Les sauvegardes quotidiennes sont vérifiées au démarrage, à la reprise et pendant l\'exécution de l\'application.';

  @override
  String get chatBackupNow => 'Sauvegarder les chats maintenant';

  @override
  String get chatBackupNever => 'Aucune sauvegarde réussie pour le moment';

  @override
  String chatBackupLastSuccess(String time) {
    return 'Dernière sauvegarde réussie : $time';
  }

  @override
  String get terminalDistributionsTitle =>
      'Télécharger des distributions Linux';

  @override
  String get terminalDistributionDownload => 'Télécharger et installer';

  @override
  String get terminalDistributionInstallAnother => 'En installer une autre';

  @override
  String get terminalInstallQueued => 'En attente d\'installation';

  @override
  String get terminalInstallConnecting => 'Test des miroirs de téléchargement…';

  @override
  String get terminalInstallCancelling => 'Annulation…';

  @override
  String get terminalInstallErrorDetails => 'Détails de l\'erreur';

  @override
  String get terminalInstallDownloading =>
      'Téléchargement du système de fichiers racine';

  @override
  String get terminalInstallVerifying => 'Vérification SHA-256';

  @override
  String get terminalInstallExtracting =>
      'Installation du système de fichiers racine';

  @override
  String get terminalInstallCompleted =>
      'Installée · disponible dans les profils de terminal';

  @override
  String get terminalInstallCancelled => 'Installation annulée';

  @override
  String get terminalInstallFailed => 'Échec de l\'installation';

  @override
  String get terminalProfileWslDistribution => 'Distribution WSL';

  @override
  String get terminalProfileWslDistributionRequired =>
      'Sélectionnez une distribution WSL.';

  @override
  String get terminalProfileWslHelp =>
      'Installez une distribution WSL sur l\'appareil Windows exécutant Core, puis rouvrez cette boîte de dialogue.';

  @override
  String get appUpdateUnavailableTitle =>
      'Mise à jour temporairement indisponible';

  @override
  String get appUpdatePlanning =>
      'Recherche de mises à jour et sélection de la source de téléchargement…';

  @override
  String get appUpdateSelectingSource =>
      'Sélection de la source de téléchargement…';

  @override
  String get appUpdateAwaitingBase =>
      'Le programme d’installation d’origine est nécessaire';

  @override
  String get appUpdateValidatingBase =>
      'Vérification du programme d’installation sélectionné…';

  @override
  String get appUpdateDownloadingPatch =>
      'Téléchargement du correctif incrémentiel…';

  @override
  String get appUpdateDownloadingFull =>
      'Téléchargement du programme d’installation complet…';

  @override
  String get appUpdateVerifyingBase =>
      'Vérification du programme d’installation d’origine…';

  @override
  String get appUpdateVerifyingPatch =>
      'Vérification du correctif incrémentiel…';

  @override
  String get appUpdateExpandingBase =>
      'Préparation du programme d’installation d’origine…';

  @override
  String get appUpdateVerifyingExpandedBase =>
      'Vérification des fichiers de mise à jour…';

  @override
  String get appUpdateApplyingPatch => 'Application du correctif…';

  @override
  String get appUpdateRestoringPackage =>
      'Création du nouveau programme d’installation…';

  @override
  String get appUpdateVerifyingPackage =>
      'Vérification du nouveau programme d’installation…';

  @override
  String get appUpdateSavingPackage =>
      'Enregistrement du programme d’installation…';

  @override
  String get appUpdatePreparingFiles =>
      'Préparation des fichiers de mise à jour…';

  @override
  String get appUpdateReady =>
      'Mise à jour téléchargée et vérifiée. Confirmez pour l’installer.';

  @override
  String get appUpdatePreparingInstall => 'Préparation de l’installation…';

  @override
  String get appUpdateInstalling =>
      'Démarrage de l’installation. L’application va bientôt redémarrer…';

  @override
  String get appUpdateRestartRequired =>
      'Téléchargement terminé. Redémarrez pour installer la nouvelle version.';

  @override
  String get appUpdateInstallSubmitted =>
      'Installation demandée. Confirmez dans la fenêtre système.';

  @override
  String get appUpdatePermissionRequired =>
      'Autorisation d’installer des applications requise';

  @override
  String get appUpdateUpdateIncomplete => 'Mise à jour incomplète';

  @override
  String get appUpdateInstallIncomplete => 'Installation incomplète';

  @override
  String get appUpdatePreparingDownload =>
      'Préparation du téléchargement de la mise à jour…';

  @override
  String get appUpdateTitle => 'Mettre à jour Operit';

  @override
  String get appUpdateNoOriginalInstaller =>
      'Je n’ai plus le programme d’installation';

  @override
  String get appUpdateChooseInstaller => 'Sélectionner';

  @override
  String get appUpdateBackground => 'Mettre à jour en arrière-plan';

  @override
  String get appUpdateCancel => 'Annuler la mise à jour';

  @override
  String get appUpdateOpenRelease => 'Ouvrir la page de la version';

  @override
  String get appUpdateContinueInstall => 'Poursuivre l’installation';

  @override
  String get appUpdateRetry => 'Réessayer la mise à jour';

  @override
  String get appUpdateReinstall => 'Réinstaller';

  @override
  String get appUpdateRestartInstall => 'Redémarrer et installer';

  @override
  String get appUpdateReopenInstaller => 'Rouvrir le programme d’installation';

  @override
  String get appUpdateRestartUpdate => 'Redémarrer et mettre à jour';

  @override
  String get appUpdateSelectOriginalInstaller =>
      'Sélectionnez le programme d’installation utilisé à l’origine. Une fois celui-ci vérifié, le correctif incrémentiel sera téléchargé et appliqué pour créer le nouveau programme d’installation.';

  @override
  String get appUpdateProcessingHelp =>
      'Les fichiers sont traités sur cet appareil. La création du programme d’installation peut prendre du temps. Vous pouvez poursuivre en arrière-plan et sélectionner le logo Operit pour suivre la progression.';

  @override
  String get appUpdateDownloadHelp =>
      'Après le téléchargement, le programme d’installation est vérifié et conservé jusqu’à votre confirmation. Vous pouvez fermer cette fenêtre et sélectionner le logo Operit pour suivre la progression.';

  @override
  String get appUpdatePermissionHelp =>
      'Accordez l’autorisation système, puis sélectionnez « Poursuivre l’installation ». Le programme d’installation a été conservé.';

  @override
  String get appUpdateReadyHelp =>
      'Sélectionnez « Redémarrer et mettre à jour » pour lancer l’installation. Le système peut demander une autorisation.';

  @override
  String get appUpdateRestartHelp =>
      'Sélectionnez « Redémarrer et installer » pour fermer l’application, installer la nouvelle version et la rouvrir.';

  @override
  String get appUpdateSystemInstallHelp =>
      'Confirmez dans le programme d’installation système, puis rouvrez Operit. Vous pouvez rouvrir le programme d’installation si vous l’avez fermé.';

  @override
  String get appUpdateInstallerKept =>
      'Le programme d’installation a été conservé. Vous pouvez réessayer sans le télécharger à nouveau.';

  @override
  String get appUpdateReleaseNotes => 'Nouveautés';

  @override
  String get appUpdateAvailableTitle => 'Mise à jour disponible';

  @override
  String get appUpdateNow => 'Mettre à jour maintenant';

  @override
  String get appUpdateAvailable =>
      'Une nouvelle version d’Operit est disponible';

  @override
  String get appUpdatePromptReady =>
      'La mise à jour est prête. Confirmez pour redémarrer et l’installer.';

  @override
  String get appUpdatePromptAvailable =>
      'Mettez à jour pour profiter des dernières fonctionnalités et améliorations.';

  @override
  String get appUpdateCurrentVersion => 'Version actuelle';

  @override
  String get appUpdateLatestVersion => 'Dernière version';

  @override
  String get appUpdatePromptLater =>
      'Vous pouvez reprendre la mise à jour plus tard en sélectionnant le logo Operit dans la barre latérale.';

  @override
  String get appUpdateOpenReleaseFailed =>
      'Impossible d’ouvrir la page de la version';

  @override
  String get appUpdateViewProgress => 'Suivre la mise à jour';

  @override
  String get appUpdateFailedRetry =>
      'Échec de la mise à jour. Sélectionnez pour réessayer.';

  @override
  String appUpdateUnavailableHelp(String version, String baseVersion) {
    return 'Le programme d’installation requis est introuvable et aucun programme complet n’est disponible. Téléchargez celui de la version $version ou retrouvez le programme d’origine de la version $baseVersion, puis réessayez.';
  }

  @override
  String appUpdateAvailableVersion(String version) {
    return 'La version $version est disponible';
  }

  @override
  String appUpdateInstalledVersion(String version) {
    return 'Version actuelle : $version';
  }

  @override
  String appUpdateOriginalVersionPrompt(String version) {
    return 'Avez-vous encore le programme d’installation d’origine de la version $version ?';
  }

  @override
  String appUpdatePatchSize(String size) {
    return 'Téléchargement incrémentiel : environ $size';
  }

  @override
  String appUpdateStepProgress(int percent) {
    return 'Étape en cours : $percent %';
  }

  @override
  String appUpdateElapsed(String duration) {
    return 'Temps écoulé : $duration';
  }

  @override
  String appUpdateDownloadSizePending(String size) {
    return '$size · Calcul de la taille du fichier';
  }

  @override
  String appUpdateBadgeAvailable(String version) {
    return 'La version $version est disponible. Sélectionnez pour mettre à jour.';
  }

  @override
  String get apiErrorInvalidParametersTitle =>
      'Paramètres de requête invalides';

  @override
  String get apiErrorReasoningContentTitle =>
      'Utilisez un fournisseur prenant en charge le renvoi du raisonnement';

  @override
  String get apiErrorReasoningContentMessage =>
      'La requête concerne reasoning_content. Dans les paramètres du modèle, sélectionnez un type de fournisseur prenant en charge le renvoi du contenu de raisonnement, comme DeepSeek ou Compatible OpenAI, et vérifiez qu’il correspond à votre API.';

  @override
  String get apiErrorInvalidParametersMessage =>
      'L’API n’a pas pu traiter les paramètres. Vérifiez le nom du modèle, ses capacités et ses paramètres, puis consultez l’erreur d’origine.';

  @override
  String get apiErrorInvalidKeyTitle => 'Clé API invalide';

  @override
  String get apiErrorInvalidKeyMessage =>
      'Le serveur a refusé la clé API. Vérifiez qu’elle est correcte, complète et active.';

  @override
  String get apiErrorBillingTitle => 'Solde API insuffisant';

  @override
  String get apiErrorBillingMessage =>
      'Votre solde est insuffisant ou votre quota est épuisé. Vérifiez votre solde et la facturation auprès du fournisseur.';

  @override
  String get apiErrorBlockedTitle => 'Requête bloquée par le serveur';

  @override
  String get apiErrorBlockedMessage =>
      'Le serveur API a refusé la requête. Contactez le fournisseur pour vérifier les restrictions de sécurité ou d’accès.';

  @override
  String get apiErrorNotFoundTitle => 'Point d’accès ou modèle introuvable';

  @override
  String get apiErrorNotFoundMessage =>
      'Le serveur API n’a pas trouvé la ressource. Vérifiez l’adresse du serveur, le chemin du point d’accès et le nom du modèle.';

  @override
  String get apiErrorUnsupportedMethodTitle =>
      'Méthode de requête non prise en charge';

  @override
  String get apiErrorUnsupportedMethodMessage =>
      'Le serveur ne prend pas en charge cette méthode ou opération. Vérifiez le type de fournisseur et le protocole API.';

  @override
  String get apiErrorUnsupportedContentTitle =>
      'Contenu de requête non pris en charge';

  @override
  String get apiErrorUnsupportedContentMessage =>
      'Le serveur n’accepte pas ce type de contenu. Vérifiez le protocole du fournisseur, le format des médias et les en-têtes personnalisés.';

  @override
  String get apiErrorProxyAuthTitle => 'Échec de l’authentification du proxy';

  @override
  String get apiErrorProxyAuthMessage =>
      'Le proxy réseau exige une authentification. Vérifiez les identifiants du proxy et les paramètres d’authentification.';

  @override
  String get apiErrorTimeoutTitle => 'Délai de la requête API dépassé';

  @override
  String get apiErrorTimeoutMessage =>
      'La requête ou une connexion entre serveurs a expiré. Vérifiez le réseau et l’état du fournisseur, puis réessayez.';

  @override
  String get apiErrorConflictTitle => 'Conflit avec l’état de la ressource';

  @override
  String get apiErrorConflictMessage =>
      'Le serveur API a signalé un conflit. Consultez l’erreur d’origine pour plus de détails.';

  @override
  String get apiErrorLengthRequiredTitle => 'Longueur du contenu requise';

  @override
  String get apiErrorLengthRequiredMessage =>
      'L’API exige que la longueur du contenu soit déclarée. Contactez le fournisseur pour confirmer les exigences.';

  @override
  String get apiErrorTooLargeTitle => 'Requête trop volumineuse';

  @override
  String get apiErrorTooLargeMessage =>
      'Le message, les pièces jointes, l’adresse ou les en-têtes dépassent la limite du serveur. Consultez l’erreur d’origine pour savoir quoi réduire.';

  @override
  String get apiErrorLockedTitle => 'Ressource verrouillée';

  @override
  String get apiErrorLockedMessage =>
      'Le serveur API a temporairement verrouillé la ressource. Réessayez plus tard ou contactez le fournisseur.';

  @override
  String get apiErrorTooEarlyTitle =>
      'Le serveur ne peut pas encore accepter la requête';

  @override
  String get apiErrorTooEarlyMessage =>
      'Le serveur a considéré la requête comme prématurée. Réessayez plus tard.';

  @override
  String get apiErrorUpgradeRequiredTitle =>
      'Mise à niveau du protocole API requise';

  @override
  String get apiErrorUpgradeRequiredMessage =>
      'Le serveur exige une autre version du protocole. Vérifiez l’adresse du serveur et le protocole configuré pour le fournisseur.';

  @override
  String get apiErrorRateLimitedTitle => 'Requête limitée ou bloquée';

  @override
  String get apiErrorRateLimitedMessage =>
      'L’API a limité ou bloqué la requête. Réessayez plus tard ou contactez le fournisseur.';

  @override
  String get apiErrorLegalRestrictionTitle =>
      'Service indisponible pour des raisons régionales ou juridiques';

  @override
  String get apiErrorLegalRestrictionMessage =>
      'Le serveur a refusé l’accès pour des raisons régionales ou juridiques. Demandez au fournisseur où le service est disponible.';

  @override
  String get apiErrorInterruptedTitle => 'Requête interrompue ou bloquée';

  @override
  String get apiErrorInterruptedMessage =>
      'La requête a été interrompue. Transmettez l’erreur d’origine au fournisseur pour vérifier un blocage, un problème de passerelle ou une connexion fermée.';

  @override
  String get apiErrorInternalTitle => 'Erreur interne du serveur API';

  @override
  String get apiErrorInternalMessage =>
      'Le serveur API a rencontré une erreur interne. Réessayez plus tard ou transmettez l’erreur d’origine au fournisseur.';

  @override
  String get apiErrorGatewayTitle => 'Réponse de passerelle API invalide';

  @override
  String get apiErrorGatewayMessage =>
      'La passerelle n’a pas reçu de réponse valide du serveur en amont. Réessayez plus tard ou contactez le fournisseur.';

  @override
  String get apiErrorUnavailableTitle =>
      'Service API temporairement indisponible';

  @override
  String get apiErrorUnavailableMessage =>
      'Le service peut être en maintenance, surchargé ou à court de capacité. Réessayez plus tard ou contactez le fournisseur.';

  @override
  String get apiErrorOriginTitle => 'Serveur d’origine de l’API inaccessible';

  @override
  String get apiErrorOriginMessage =>
      'La passerelle n’a pas pu joindre le serveur d’origine. Contactez le fournisseur pour vérifier une panne ou un problème de routage.';

  @override
  String get apiErrorTlsTitle => 'Erreur TLS ou de certificat du serveur API';

  @override
  String get apiErrorTlsMessage =>
      'La négociation TLS ou la validation du certificat a échoué entre la passerelle et le serveur d’origine. Contactez le fournisseur.';

  @override
  String get apiErrorStorageTitle => 'Stockage du serveur API insuffisant';

  @override
  String get apiErrorStorageMessage =>
      'Le serveur API manque d’espace pour traiter la requête. Contactez le fournisseur.';

  @override
  String get apiErrorClientTitle => 'Le serveur API a refusé la requête';

  @override
  String get apiErrorClientMessage =>
      'Le serveur API a renvoyé une erreur de requête. Consultez l’erreur d’origine et vérifiez la configuration de l’API.';

  @override
  String get apiErrorServerTitle => 'Erreur du serveur API';

  @override
  String get apiErrorServerMessage =>
      'Le serveur n’a pas pu traiter la requête. Transmettez le code HTTP et l’erreur d’origine au fournisseur.';

  @override
  String apiErrorHttpTitle(int status, String title) {
    return 'HTTP $status : $title';
  }

  @override
  String apiErrorOriginal(String error) {
    return 'Erreur d’origine :\n$error';
  }

  @override
  String get apiErrorModelExistsTitle => 'Le modèle existe déjà';

  @override
  String get apiErrorModelListTitle => 'Impossible de récupérer les modèles';

  @override
  String get apiErrorModelListMessage =>
      'Aucune liste de modèles n’a été reçue. Vérifiez l’adresse du fournisseur, la clé d’accès et la connexion réseau.';

  @override
  String get apiErrorNetworkTitle => 'Échec de la connexion réseau';

  @override
  String get apiErrorNetworkMessage =>
      'Impossible de joindre le fournisseur du modèle. Vérifiez la connexion réseau et l’adresse du serveur.';

  @override
  String get apiErrorModelConfigTitle => 'Échec de la configuration du modèle';

  @override
  String get apiErrorModelConfigMessage =>
      'La configuration du modèle a échoué. Consultez l’erreur d’origine pour plus de détails.';

  @override
  String apiErrorModelExistsMessage(String modelId, String providerName) {
    return 'Le modèle « $modelId » a déjà été ajouté au fournisseur « $providerName ».';
  }

  @override
  String get chatCopyMessage => 'Copier le message';

  @override
  String get chatEditResend => 'Modifier et renvoyer';

  @override
  String get chatRollbackHere => 'Revenir à cet état';

  @override
  String get chatRegenerate => 'Régénérer';

  @override
  String get chatEditMemory => 'Modifier le souvenir';

  @override
  String get chatPlaySpeech => 'Générer ou lire l’audio';

  @override
  String get chatDeleteVariant => 'Supprimer la variante actuelle';

  @override
  String get chatReply => 'Répondre';

  @override
  String get chatInsertSummary => 'Insérer un résumé';

  @override
  String get chatCreateBranch => 'Créer une branche';

  @override
  String get chatMultiSelect => 'Sélectionner des messages';

  @override
  String get chatMultiSelectExit => 'Quitter la sélection';

  @override
  String get chatSelectionClear => 'Effacer la sélection';

  @override
  String get chatSelectionAll => 'Tout sélectionner';

  @override
  String get chatSelectionCopy => 'Copier les messages sélectionnés';

  @override
  String get chatGenerateShareImage => 'Créer une image de partage';

  @override
  String get chatSelectionDelete => 'Supprimer les messages sélectionnés';

  @override
  String get chatEditMessage => 'Modifier le message';

  @override
  String get chatEditorVisual => 'Visuel';

  @override
  String get chatPlainText => 'Texte brut';

  @override
  String get chatMarkdownSource => 'Source Markdown';

  @override
  String get chatCopyPlainText => 'Copier le texte brut';

  @override
  String get chatCopyMarkdown => 'Copier la source Markdown';

  @override
  String get chatMessageCopied => 'Message copié dans le presse-papiers';

  @override
  String get chatSaveResend => 'Enregistrer et renvoyer';

  @override
  String get chatUpdateMemory => 'Mettre à jour le souvenir';

  @override
  String get chatEditorPlainTextContent => 'Contenu en texte brut';

  @override
  String get chatEditorParts => 'Blocs de contenu';

  @override
  String get chatEditorAddText => 'Ajouter du texte';

  @override
  String get chatEditorAddTag => 'Ajouter une balise';

  @override
  String get chatEditorText => 'Texte';

  @override
  String get chatEditorTextHint => 'Saisir du texte';

  @override
  String get chatEditorEditTag => 'Modifier la balise';

  @override
  String get chatEditorTagName => 'Nom de la balise';

  @override
  String get chatEditorTagHint => 'Par exemple, custom_tag';

  @override
  String get chatEditorAttributes => 'Attributs (facultatif)';

  @override
  String get chatEditorAttributesHint => 'Par exemple, type=\"note\"';

  @override
  String get chatEditorContent => 'Contenu';

  @override
  String get chatRollbackConfirmTitle =>
      'Restaurer l’état de l’espace de travail ?';

  @override
  String get chatResendConfirmTitle => 'Modifier et renvoyer le message ?';

  @override
  String get chatRollbackConfirmMessage =>
      'Cette action rétablit l’état de l’espace de travail avant ce message et supprime ce message ainsi que les suivants.';

  @override
  String get chatResendConfirmMessage =>
      'Cette action rétablit l’état de l’espace de travail avant ce message, puis renvoie le message modifié.';

  @override
  String get chatRollbackConfirm => 'Rétablir l’état précédent';

  @override
  String get chatNoWorkspaceChanges =>
      'Aucune modification des fichiers de l’espace de travail détectée.';

  @override
  String get chatShareImageGenerating => 'Création de l’image de partage…';

  @override
  String chatSelectionCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count messages sélectionnés',
      one: '$count message sélectionné',
    );
    return '$_temp0';
  }

  @override
  String chatSelectionDeleteConfirm(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Supprimer les $count messages sélectionnés ?',
      one: 'Supprimer le message sélectionné ?',
    );
    return '$_temp0';
  }

  @override
  String chatCopyFailed(String error) {
    return 'Échec de la copie : $error';
  }

  @override
  String chatPlainTextConversionFailed(String error) {
    return 'Impossible de convertir en texte brut : $error';
  }

  @override
  String chatPlaySpeechFailed(String error) {
    return 'Impossible de générer ou de lire l’audio : $error';
  }

  @override
  String chatShareImageFailed(String error) {
    return 'Impossible de créer l’image de partage : $error';
  }

  @override
  String chatWorkspaceChangeLines(String change, int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count lignes',
      one: '$count ligne',
    );
    return '$change · $_temp0';
  }

  @override
  String get speechTitle => 'Lecture à voix haute';

  @override
  String get speechIdle => 'En attente';

  @override
  String get speechPreparing => 'Génération de l’audio';

  @override
  String get speechPlaying => 'Lecture en cours';

  @override
  String get speechStopped => 'Arrêtée';

  @override
  String get speechError => 'Erreur';

  @override
  String speechQueue(int count) {
    return 'File d’attente : $count';
  }

  @override
  String get speechTestText => 'Bonjour ! Voici un aperçu de la voix d’Operit.';

  @override
  String get speechTtsMissingId =>
      'Impossible de lancer l’aperçu sans identifiant de configuration TTS';

  @override
  String get speechTtsPreviewPlaying => 'L’aperçu TTS a démarré';

  @override
  String get speechTtsProviders => 'Fournisseurs TTS';

  @override
  String get speechAddTtsProvider => 'Ajouter un fournisseur TTS';

  @override
  String get speechSttCurrentCannotDelete =>
      'La configuration STT active ne peut pas être supprimée';

  @override
  String get speechDeleteSttProvider => 'Supprimer le fournisseur STT';

  @override
  String get speechSttProviders => 'Fournisseurs STT';

  @override
  String get speechAddSttProvider => 'Ajouter un fournisseur STT';

  @override
  String get speechEditProvider => 'Modifier le fournisseur';

  @override
  String get speechPreview => 'Écouter un aperçu';

  @override
  String get speechGlobalActive => 'Valeur globale par défaut';

  @override
  String get speechSetGlobal => 'Définir comme valeur globale par défaut';

  @override
  String get speechNoSttProviders =>
      'Aucun fournisseur de reconnaissance vocale configuré';

  @override
  String get speechCurrentCannotDelete =>
      'La configuration active ne peut pas être supprimée';

  @override
  String get speechEditTtsProvider => 'Modifier le fournisseur TTS';

  @override
  String get speechProviderType => 'Type de fournisseur';

  @override
  String get speechProviderName => 'Nom du fournisseur';

  @override
  String get speechHttpMethod => 'Méthode HTTP';

  @override
  String get speechBodyTemplate => 'Modèle du corps de requête';

  @override
  String get speechPipelineJson => 'JSON du pipeline de réponse';

  @override
  String get speechAddTtsVoice => 'Ajouter une voix TTS';

  @override
  String get speechCustomVoice => 'Voix personnalisée';

  @override
  String get speechEnterVoice => 'Saisir le modèle et la voix manuellement';

  @override
  String get speechModelVoiceRequired => 'Saisissez un modèle ou une voix';

  @override
  String get speechCustomTtsVoice => 'Voix TTS personnalisée';

  @override
  String get speechModelRequired => 'Le modèle est obligatoire';

  @override
  String get speechVoice => 'Voix';

  @override
  String get speechVoiceRequired => 'La voix est obligatoire';

  @override
  String get speechLanguageTag =>
      'Balise de langue (facultatif, par ex. fr-FR)';

  @override
  String get speechModelLocale => 'Modèle / paramètres régionaux';

  @override
  String get speechSystemVoiceName => 'Nom de la voix système (facultatif)';

  @override
  String get speechEditTtsVoice => 'Modifier la voix TTS';

  @override
  String get speechTestVoice => 'Tester la voix';

  @override
  String get speechAudioFormat => 'Format audio';

  @override
  String get speechSpeed => 'Vitesse';

  @override
  String get speechNewTtsProvider => 'Nouveau fournisseur TTS';

  @override
  String get speechModelVersion => 'ID du modèle@version';

  @override
  String get speechNewSttProvider => 'Nouveau fournisseur STT';

  @override
  String get speechEditSttProvider => 'Modifier le fournisseur STT';

  @override
  String get speechMultipart => 'Paramètres de la requête multipart';

  @override
  String get speechAudioField => 'Nom du champ audio';

  @override
  String get speechModelField => 'Nom du champ du modèle';

  @override
  String get speechModelFieldHelp =>
      'Laissez vide si le point d’accès n’accepte pas de paramètre de modèle';

  @override
  String get speechLanguageField => 'Nom du champ de langue';

  @override
  String get speechTranscriptJsonPath => 'JSONPath du texte transcrit';

  @override
  String get speechModelNotNeeded =>
      'Ce point d’accès ne nécessite pas de modèle';

  @override
  String get speechReloadModels => 'Recharger les modèles';

  @override
  String get speechModelIdHelp =>
      'Saisissez un ID de modèle pris en charge ou sélectionnez un préréglage';

  @override
  String get speechPresetModel => 'Choisir un modèle prédéfini';

  @override
  String get speechNoLocalSttModels => 'Aucun modèle STT local installé';

  @override
  String get speechChooseModel => 'Choisissez un modèle';

  @override
  String get speechChooseSttModel => 'Choisissez un modèle STT';

  @override
  String speechPreviewTitle(String voice) {
    return 'Aperçu · $voice';
  }

  @override
  String speechPreviewFailed(String error) {
    return 'Échec de l’aperçu TTS : $error';
  }

  @override
  String speechDeleteTtsConfigFailed(String error) {
    return 'Impossible de supprimer la configuration TTS : $error';
  }

  @override
  String speechLoadConfigFailed(String error) {
    return 'Impossible de charger les paramètres vocaux : $error';
  }

  @override
  String speechDeleteSttConfirm(String name) {
    return 'Supprimer « $name » et ses paramètres de reconnaissance ?';
  }

  @override
  String speechSttFailed(String error) {
    return 'Échec de l’opération STT : $error';
  }

  @override
  String speechModelsFailed(String error) {
    return 'Impossible de charger les modèles : $error';
  }

  @override
  String speechFieldRequired(String label) {
    return 'Champ obligatoire : $label';
  }

  @override
  String speechFieldPositive(String label) {
    return 'La valeur doit être positive : $label';
  }

  @override
  String get onboardingStorageRequired =>
      'Les répertoires du moteur et de l’espace de travail sont obligatoires';

  @override
  String get onboardingFetchModelsFirst =>
      'Récupérez les modèles disponibles avant de continuer';

  @override
  String get onboardingChooseDefaultModel => 'Choisissez un modèle par défaut';

  @override
  String get onboardingPickSnapshot => 'Choisir un instantané';

  @override
  String get onboardingPickSnapshotHelp =>
      'Sélectionnez un instantané Operit1 dans le sélecteur de fichiers.';

  @override
  String get onboardingUploadSnapshot => 'Téléverser l’instantané';

  @override
  String get onboardingInspectSnapshot => 'Examiner l’instantané';

  @override
  String get onboardingInspectSnapshotHelp =>
      'Examen du contenu de l’instantané Operit1…';

  @override
  String get onboardingSnapshotRequired =>
      'Sélectionnez un fichier d’instantané Operit1';

  @override
  String get onboardingPrepareImport => 'Préparer l’importation';

  @override
  String get onboardingPrepareMigration =>
      'Préparation de la migration du contenu de l’instantané Operit1…';

  @override
  String get onboardingWait => 'Veuillez patienter';

  @override
  String get onboardingAgree => 'J’accepte';

  @override
  String get onboardingSaving => 'Enregistrement…';

  @override
  String get onboardingConfirm => 'Confirmer';

  @override
  String get onboardingPreparing => 'Préparation…';

  @override
  String get onboardingContinue => 'Continuer';

  @override
  String get onboardingFetching => 'Récupération…';

  @override
  String get onboardingReading => 'Lecture…';

  @override
  String get onboardingImporting => 'Importation…';

  @override
  String get onboardingWorking => 'Traitement…';

  @override
  String get onboardingDone => 'Terminer';

  @override
  String get onboardingAgreement => 'Conditions d’utilisation';

  @override
  String get onboardingStorage => 'Emplacements de stockage';

  @override
  String get onboardingMode => 'Mode de configuration';

  @override
  String get onboardingModelSetup => 'Configuration du modèle';

  @override
  String get onboardingImportSetup => 'Importer la configuration';

  @override
  String get onboardingDeviceSpace => 'Espace d’appareils';

  @override
  String get onboardingPermissions => 'Autorisations système';

  @override
  String get onboardingWelcome => 'Bienvenue';

  @override
  String get onboardingPreparingRuntime => 'Préparation du moteur local';

  @override
  String get onboardingSkip => 'Passer';

  @override
  String get onboardingPreviousPage => 'Page précédente';

  @override
  String get onboardingStart => 'Commencer';

  @override
  String get onboardingTagline => 'Simplifiez vos tâches quotidiennes';

  @override
  String get onboardingChooseMode => 'Choisissez comment commencer';

  @override
  String get onboardingChooseModeHelp =>
      'Configurez un nouvel environnement, importez des données d’Operit1, rejoignez un espace d’appareils ou restaurez vos discussions depuis une sauvegarde.';

  @override
  String get onboardingQuickStart => 'Démarrage rapide';

  @override
  String get onboardingQuickStartHelp =>
      'Configurez un fournisseur de modèles pour terminer la configuration de base';

  @override
  String get onboardingImportLegacyHelp =>
      'Importez les paramètres et les données de la version précédente';

  @override
  String get onboardingJoinExisting =>
      'Rejoindre un espace d’appareils existant';

  @override
  String get onboardingJoinExistingHelp =>
      'Trouvez un espace d’appareils à proximité et synchronisez les modèles et les données après l’avoir rejoint';

  @override
  String get onboardingJoinSpace => 'Rejoindre un espace d’appareils';

  @override
  String get onboardingJoinSpaceHelp =>
      'Sélectionnez « Ajouter un appareil », choisissez un appareil à proximité et associez-le. Les paramètres et les données sont synchronisés après l’approbation d’un administrateur.';

  @override
  String get onboardingConfirmStorage =>
      'Confirmer les emplacements de stockage locaux';

  @override
  String get onboardingConfirmStorageHelp =>
      'Les données du moteur et celles de l’espace de travail sont stockées séparément. Saisissez les chemins ou choisissez chaque répertoire.';

  @override
  String get onboardingRuntimeDirectory => 'Répertoire du moteur';

  @override
  String get onboardingWorkspaceDirectory =>
      'Répertoire de l’espace de travail';

  @override
  String get onboardingReadingPaths => 'Lecture des chemins de stockage';

  @override
  String get onboardingStorageMountHelp =>
      'Ces répertoires stockent séparément l’état du moteur et les espaces de travail. L’hôte local les monte après votre confirmation.';

  @override
  String get onboardingPrepareSnapshotHelp =>
      'Préparation de l’instantané Operit1…';

  @override
  String get onboardingImportSnapshotHelp =>
      'Utilisez un instantané exporté par la dernière version d’Operit1 pour migrer les paramètres, les discussions, les fiches de personnage et les ressources vers Operit2.';

  @override
  String get onboardingReadingSnapshot => 'Lecture de l’instantané';

  @override
  String get onboardingPickSnapshotFile => 'Choisir le fichier d’instantané';

  @override
  String get onboardingMigrationContents => 'Données à migrer';

  @override
  String get onboardingChats => 'Discussions';

  @override
  String get onboardingMessages => 'Messages';

  @override
  String get onboardingPreferenceFiles => 'Fichiers de préférences';

  @override
  String get onboardingResourceFiles => 'Fichiers de ressources';

  @override
  String get onboardingExternalResources => 'Ressources externes';

  @override
  String get onboardingImportInProgress => 'Importation de l’instantané…';

  @override
  String get onboardingImportContinueHelp =>
      'Sélectionnez « Continuer » pour migrer l’intégralité de l’instantané.';

  @override
  String get onboardingEndpoint => 'Adresse du serveur';

  @override
  String get onboardingCompleteModel => 'Terminer la configuration du modèle';

  @override
  String get onboardingCompleteModelHelp =>
      'Choisissez un fournisseur, saisissez votre clé API, puis récupérez et sélectionnez un modèle par défaut.';

  @override
  String get onboardingModelProvider => 'Fournisseur de modèles';

  @override
  String get onboardingModelProviderRequired =>
      'Choisissez un fournisseur de modèles';

  @override
  String get onboardingContinueModel => 'Poursuivre la configuration';

  @override
  String get onboardingFetchingModels => 'Récupération des modèles';

  @override
  String get onboardingFetchModels => 'Récupérer les modèles disponibles';

  @override
  String get onboardingDefaultModel => 'Modèle par défaut';

  @override
  String get onboardingGrantPermissions =>
      'Accorder les autorisations nécessaires';

  @override
  String get onboardingGrantPermissionsHelp =>
      'Accordez les autorisations des fonctionnalités que vous souhaitez utiliser. Vous pouvez continuer sans rien accorder. Les fonctionnalités qui nécessitent ces autorisations seront indisponibles.';

  @override
  String get onboardingNoPermissions =>
      'Aucune autorisation à vérifier sur cet appareil';

  @override
  String get onboardingNoPermissionsHelp =>
      'Aucune autorisation système ne doit être configurée dans cet environnement.';

  @override
  String get onboardingRefreshPermissions => 'Actualiser les autorisations';

  @override
  String get onboardingOptional => 'Facultatif';

  @override
  String get onboardingRequired => 'Champ obligatoire';

  @override
  String get onboardingPermissionGranted => 'Accordée';

  @override
  String get onboardingGrantPermission => 'Accorder l’autorisation';

  @override
  String get onboardingNoAction => 'Aucune action nécessaire';

  @override
  String onboardingUploadProgress(int percent) {
    return 'Lecture et téléversement de l’instantané ($percent %).';
  }

  @override
  String onboardingChooseDirectory(String label) {
    return 'Choisir : $label';
  }

  @override
  String onboardingDefaultChatModel(String model) {
    return 'Modèle de discussion par défaut : $model';
  }

  @override
  String onboardingImportFailed(String error) {
    return 'Échec de l’importation de l’instantané\n$error';
  }

  @override
  String get onboardingAgreementTitle =>
      'Conditions d’utilisation et politique de confidentialité';

  @override
  String get onboardingAgreementHelp =>
      'Lisez et acceptez ces conditions pour poursuivre la configuration.';

  @override
  String onboardingAgreementVersion(String version) {
    return 'Version : $version';
  }

  @override
  String onboardingAgreementWait(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Vous pourrez accepter dans $count secondes.',
      one: 'Vous pourrez accepter dans $count seconde.',
    );
    return '$_temp0';
  }

  @override
  String get extensionsView => 'Afficher';

  @override
  String get extensionsCloseSearch => 'Fermer la recherche';

  @override
  String get extensionsSearchShortcut => 'Rechercher (Ctrl+F)';

  @override
  String get extensionsAdd => 'Ajouter une extension';

  @override
  String get extensionsMore => 'Plus';

  @override
  String get extensionsCheckUpdates => 'Rechercher des mises à jour';

  @override
  String get extensionsEnvironment => 'Variables d’environnement';

  @override
  String get extensionsReload => 'Recharger le catalogue';

  @override
  String get extensionsSearchInstalled =>
      'Rechercher parmi les extensions installées';

  @override
  String get extensionsSearchMarket =>
      'Rechercher dans le marché et les extensions officielles';

  @override
  String get extensionsTab => 'Extensions';

  @override
  String get extensionsDrop => 'Déposez pour importer des extensions';

  @override
  String get extensionsDropTypes =>
      'Formats pris en charge : .toolpkg, .js, .ts et .hjson';

  @override
  String get extensionsImportPlugin => 'Importer un plugin';

  @override
  String get extensionsSelectToolPkg => 'Choisir un fichier .toolpkg';

  @override
  String get extensionsImportScript => 'Importer un paquet de scripts';

  @override
  String get extensionsScriptTypes => '.js, .ts, .hjson ou .toolpkg';

  @override
  String get extensionsAddSkill => 'Ajouter une compétence';

  @override
  String get extensionsSkillSources => 'Dépôt GitHub, ZIP ou saisie manuelle';

  @override
  String get extensionsAddMcp => 'Ajouter un serveur MCP';

  @override
  String get extensionsMcpSources => 'GitHub, ZIP, configuration ou formulaire';

  @override
  String get extensionsAddA2a => 'Ajouter un agent A2A';

  @override
  String get extensionsA2aSources =>
      'Adresse du serveur, Agent Card ou fichier de configuration';

  @override
  String get extensionsAiCreate => 'Créer un plugin avec l’IA';

  @override
  String get extensionsAiCreateHelp =>
      'Décrivez votre besoin, puis générez et installez le plugin depuis la discussion';

  @override
  String get extensionsAddHelp =>
      'Les nouvelles extensions sont installées sur cet appareil par défaut. Un ToolPkg peut déclarer un périmètre d’installation fixe. La mise à jour d’un ToolPkg local portant le même ID conserve son emplacement et sa configuration. Sur ordinateur, vous pouvez aussi glisser-déposer les fichiers.';

  @override
  String extensionsSyncChanged(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Espace d’appareils synchronisé. $count extensions ont changé.',
      one: 'Espace d’appareils synchronisé. Une extension a changé.',
    );
    return '$_temp0';
  }

  @override
  String extensionsDropRejected(String files) {
    return 'Importation directe impossible : $files. Utilisez « Ajouter une extension » pour importer des compétences ou des serveurs MCP.';
  }

  @override
  String get marketRangeWeek => 'Dernière semaine';

  @override
  String get marketRangeMonth => 'Dernier mois';

  @override
  String get marketRangeAll => 'Depuis toujours';

  @override
  String get marketSortUpdated => 'Mis à jour récemment';

  @override
  String get marketSortDownloads => 'Les plus téléchargés';

  @override
  String get marketSortLikes => 'Les plus populaires';

  @override
  String marketFilterSummary(String sort, String range) {
    return 'Filtres : $sort, $range';
  }

  @override
  String get marketFilterTitle => 'Filtrer le marché';

  @override
  String get marketFilterSort => 'Trier par';

  @override
  String get marketFilterPeriod => 'Période';

  @override
  String get marketFilterUpdatedHelp =>
      'Filtrez selon la date de dernière mise à jour.';

  @override
  String get marketFilterCategory => 'Catégorie';

  @override
  String get marketFilterAllCategories => 'Toutes les catégories';

  @override
  String get marketFilterCategoryRetry =>
      'Impossible de charger les catégories. Réessayer.';

  @override
  String get marketFilterReset => 'Réinitialiser';

  @override
  String get marketFilterApply => 'Appliquer les filtres';

  @override
  String marketFilterRankingHelp(String metric) {
    return 'Par défaut, le classement porte sur les contenus publiés pour la première fois au cours des 30 derniers jours, selon leur nombre total de $metric. Les mises à jour ne modifient pas la date de publication.';
  }

  @override
  String get marketFilterLikes => 'mentions J’aime';

  @override
  String get marketFilterDownloads => 'téléchargements';

  @override
  String get attachmentContext => 'Contexte supplémentaire';

  @override
  String get attachmentExtraInfo => 'Informations supplémentaires';

  @override
  String get attachmentPastedText => 'Texte collé';

  @override
  String get attachmentResetZoom => 'Réinitialiser le zoom';

  @override
  String get attachmentCloseShortcut => 'Fermer (Échap)';

  @override
  String get attachmentCopyContent => 'Copier le contenu';

  @override
  String get attachmentCopied => 'Copié dans le presse-papiers';

  @override
  String get attachmentCopyLocation => 'Copier l’emplacement du fichier';

  @override
  String get attachmentClipboardImage => 'Image du presse-papiers';

  @override
  String get chatSharePreview => 'Aperçu de l’image à partager';

  @override
  String get chatShareSaveFile => 'Enregistrer dans un fichier';

  @override
  String get chatShareImage => 'Partager l’image';

  @override
  String get chatShareSaveFailed => 'Impossible d’enregistrer';

  @override
  String get chatSharePng => 'Image PNG';

  @override
  String get chatShareSaved => 'Image enregistrée';

  @override
  String get chatShareFailed => 'Impossible de partager';

  @override
  String attachmentExpandCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Afficher les $count pièces jointes',
      one: 'Afficher la pièce jointe',
    );
    return '$_temp0';
  }

  @override
  String attachmentCollapseCount(int count) {
    return 'Réduire ($count)';
  }

  @override
  String attachmentContextCount(int count) {
    return 'Contexte supplémentaire · $count';
  }

  @override
  String attachmentCopyLocationPath(String path) {
    return 'Copier l’emplacement du fichier\n$path';
  }

  @override
  String attachmentLocationPath(String path) {
    return 'Enregistré dans : $path';
  }

  @override
  String get marketCategorySearch => 'Recherche et études';

  @override
  String get marketCategorySearchDescription =>
      'Moteurs de recherche, outils d’étude et recherche de connaissances';

  @override
  String get marketCategoryDevelopment => 'Outils de développement';

  @override
  String get marketCategoryDevelopmentDescription =>
      'Outils d’édition de code, de débogage et de développement';

  @override
  String get marketCategoryAutomation => 'Automatisation';

  @override
  String get marketCategoryAutomationDescription =>
      'Automatisation des tâches et flux de travail';

  @override
  String get marketCategoryDocuments => 'Documents et connaissances';

  @override
  String get marketCategoryDocumentsDescription =>
      'Traitement de documents et gestion des connaissances';

  @override
  String get marketCategoryMedia => 'Médias et contenu';

  @override
  String get marketCategoryMediaDescription =>
      'Traitement des images, vidéos et sons, et création de contenu';

  @override
  String get marketCategoryCommunication => 'Discussion et communication';

  @override
  String get marketCategoryCommunicationDescription =>
      'Outils de discussion, messagerie et intégrations de communication';

  @override
  String get marketCategoryIntegration => 'Services et API';

  @override
  String get marketCategoryIntegrationDescription =>
      'Intégrations de services tiers et outils API';

  @override
  String get marketCategorySystem => 'Système et données';

  @override
  String get marketCategorySystemDescription =>
      'Systèmes de fichiers, gestion des données et outils système';

  @override
  String get marketCategoryProductivity =>
      'Activité professionnelle et productivité';

  @override
  String get marketCategoryProductivityDescription =>
      'Flux de travail professionnels et outils de productivité';

  @override
  String get marketCategoryEntertainment => 'Vie quotidienne et loisirs';

  @override
  String get marketCategoryEntertainmentDescription =>
      'Utilitaires du quotidien et applications de divertissement';

  @override
  String get marketCategoryOther => 'Autres';

  @override
  String get marketCategoryOtherDescription =>
      'Extensions non classées ou d’autres types';

  @override
  String get settingsCategorySpeechTitle => 'Voix et reconnaissance';

  @override
  String get settingsCategorySpeechSubtitle => 'Fournisseurs TTS et STT';

  @override
  String get settingsCategorySpeechDescription =>
      'Gérez les fournisseurs de synthèse et de reconnaissance vocales, puis choisissez la configuration active pour chacun.';

  @override
  String get settingsCategoryAboutTitle => 'À propos d’Operit2';

  @override
  String get settingsCategoryAboutSubtitle => 'Version, projet et licence';

  @override
  String get settingsCategoryAboutDescription =>
      'Consultez la version d’Operit2, les liens du projet et les licences libres.';

  @override
  String get aboutUpdateSection => 'Mises à jour';

  @override
  String get aboutAppDescription =>
      'Un espace de travail IA pour le terminal et le bureau, avec des discussions, espaces de travail, outils, plugins, MCP, connexions distantes et accès Web.';

  @override
  String get aboutReadingVersion => 'Lecture des informations de version…';

  @override
  String get aboutUpToDate => 'Vous utilisez la dernière version';

  @override
  String get aboutCheckingUpdates => 'Recherche de mises à jour…';

  @override
  String get aboutRetryCheck =>
      'Échec de la recherche de mises à jour. Sélectionnez pour réessayer.';

  @override
  String get aboutCheckHint => 'Sélectionnez pour rechercher une mise à jour';

  @override
  String get aboutProject => 'Projet';

  @override
  String get aboutSource => 'Code source du projet';

  @override
  String get aboutDocs => 'Documentation';

  @override
  String get aboutDocsSubtitle => 'README et guide de la ligne de commande';

  @override
  String get aboutLicenses => 'Licences libres';

  @override
  String get aboutLicenseSubtitle =>
      'Operit2 est distribué sous licence AGPL-3.0';

  @override
  String get aboutContact => 'Contact';

  @override
  String get aboutDeveloper => 'Développeur AAswordman';

  @override
  String get aboutCopyright => '© 2025–2026 Operit. Tous droits réservés.';

  @override
  String aboutReadVersionFailed(String error) {
    return 'Impossible de lire les informations de version : $error';
  }

  @override
  String aboutVersion(String version) {
    return 'Version $version';
  }

  @override
  String aboutUpdatingTo(String version) {
    return 'Mise à jour vers $version. Sélectionnez pour suivre la progression.';
  }

  @override
  String aboutLinkCopied(String url) {
    return 'Aucune application ne peut ouvrir ce lien. Copié dans le presse-papiers : $url';
  }

  @override
  String get settingsToolsAdvanced => 'Paramètres avancés';

  @override
  String get settingsToolsReadOnly => 'Lecture seule';

  @override
  String get settingsToolsReadWrite => 'Lecture et écriture';

  @override
  String get settingsToolsFullAccess => 'Accès complet';

  @override
  String get settingsToolsSandboxed => 'Isolé (bac à sable macOS activé)';

  @override
  String get settingsToolsUnsandboxed =>
      'Non isolé (bac à sable macOS désactivé)';

  @override
  String get settingsToolsIsolation => 'Isolation de l’application';

  @override
  String get settingsToolsRestrictions => 'Restrictions des capacités de l’IA';

  @override
  String get settingsToolsConfirmCalls => 'Confirmation des appels d’outils';

  @override
  String get settingsToolsConfirmDirectCalls =>
      'L’approbation s’applique aux outils que l’IA appelle directement.';

  @override
  String get settingsToolsRuntimeEnvironment =>
      'Environnement d’exécution actuel';

  @override
  String get settingsToolsPlatform => 'Plateforme';

  @override
  String get settingsToolsOuterIsolation => 'Isolation de l’hôte';

  @override
  String get settingsToolsFileCapability => 'Accès aux fichiers';

  @override
  String get settingsToolsTerminalCapability => 'Accès au terminal';

  @override
  String get settingsToolsRegistered => 'Enregistré';

  @override
  String get settingsToolsUnregistered => 'Non enregistré';

  @override
  String get settingsToolsAuthorizations => 'Autorisations';

  @override
  String get settingsToolsStructuredCapabilities => 'Capacités structurées';

  @override
  String get settingsToolsNotGranted => 'Non accordée';

  @override
  String get settingsToolsSystemRequired => 'Action système requise';

  @override
  String get settingsToolsOpenSettings => 'Ouvrir les paramètres';

  @override
  String get settingsToolsUnavailable => 'Indisponible';

  @override
  String get settingsToolsHandle => 'Vérifier';

  @override
  String get settingsToolsReadOnlyDescription =>
      'L’IA peut lire l’espace de travail actuel et ne peut pas exécuter d’outils qui y écrivent.';

  @override
  String get settingsToolsReadWriteDescription =>
      'L’IA peut lire et modifier l’espace de travail actuel. Le bac à sable de l’application reste activé.';

  @override
  String get settingsToolsFullAccessDescription =>
      'L’IA peut lire et modifier l’espace de travail actuel. Ce mode désactive le bac à sable de l’application.';

  @override
  String settingsToolsAuthorizationFailed(String error) {
    return 'Échec de l’autorisation : $error';
  }

  @override
  String settingsToolsItemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count éléments',
      one: '$count élément',
    );
    return '$_temp0';
  }
}
