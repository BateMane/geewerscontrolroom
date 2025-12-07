# Geewer's Control Room

![Version](https://img.shields.io/badge/version-1.0.4-blue?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-2.0-orange?style=flat-square&logo=tauri)
![Svelte](https://img.shields.io/badge/Svelte-red?style=flat-square&logo=svelte)
![Platform](https://img.shields.io/badge/platform-Windows-blueviolet?style=flat-square&logo=windows)

Salut ! **Geewer's Control Room**, c'est ta station de contrôle ultime pour garder un œil sur ton PC.

J'en avais marre du Gestionnaire des tâches moche ou des logiciels usine à gaz pour voir si mon PC chauffait. J'ai créé cette app pour avoir un dashboard stylé, fluide et précis. Elle surveille tes composants, teste tes performances et te donne les infos essentielles en un clin d'œil.

C'est léger, ça ne bouffe pas tes FPS, et c'est codé avec **Tauri (Rust)** pour une rapidité maximale.

<img width="100%" alt="interface" src="preview.png" />

---

## 🇫🇷 Fonctionnalités

L'idée est de rassembler tout ce qu'il faut savoir sur ta machine au même endroit :

* **Tableau de Bord Temps Réel** : Surveille la charge CPU, l'utilisation de la RAM, le Swap et surtout ton débit réseau actuel (Upload/Download) à la seconde près.
* **Santé du PC** : Une note globale sur 100 calculée en direct pour savoir si ton système est en souffrance.
* **Outils Intégrés** :
    * 🚀 **Dernières Activités** : Vois quelles applications tu as lancées récemment.
    * 🧪 **Benchmark CPU** : Teste la puissance brute de ton processeur avec un calcul intensif.
    * 🌐 **Test Latence** : Un ping rapide vers Google pour vérifier si ta connexion lag.
* **Specs Détaillées** : Détection intelligente de ta "vraie" carte graphique (ignore les drivers virtuels), liste de tes disques avec barres d'espace, et tri de tes périphériques (Écrans, Audio, Claviers...).
* **C'est ton interface** : Change la couleur d'accentuation, le fond des cartes et le thème global pour matcher ton setup.

---

## 🛠️ Comment l'installer ou le tester ?

Si tu veux tester le projet ou modifier le code, c'est assez simple. Il te faut juste **Node.js** et **Rust** installés sur ta machine.

1.  **Récupère le projet**
    ```bash
    git clone [https://github.com/BateMane/geewerscontrolroom.git](https://github.com/BateMane/geewerscontrolroom.git)
    cd geewer-hw-monitor
    ```

2.  **Installe ce qu'il faut**
    ```bash
    npm install
    ```

3.  **Lance l'app**
    ```bash
    npm run tauri dev
    ```

Et voilà, la salle de contrôle est ouverte !

---

## 💻 C'est fait comment ?

Pour les curieux, voici la stack technique :
* **Frontend** : Svelte + Tailwind CSS (pour le design Cyberpunk fluide).
* **Backend** : Rust via Tauri (pour les performances et l'accès bas niveau au matériel via `sysinfo` et `PowerShell`).

Si tu as des idées pour améliorer le tool ou si tu trouves un bug, n'hésite pas à ouvrir une *Issue* ou à utiliser le bouton de feedback directement dans les paramètres de l'app !

---

<br>

# 🇬🇧 English Description

**Geewer's Control Room** is your ultimate dashboard to monitor your PC stats with style.

Tired of the ugly Task Manager? This app provides a sleek, Cyberpunk-inspired interface to monitor your hardware in real-time without eating up your resources.

### ⚡ What can it do?

* **Real-time Dashboard:** Monitor CPU, RAM, Swap, and Network bandwidth (Up/Down) live.
* **System Health:** A live global score out of 100 based on system load.
* **Built-in Tools:**
    * 🚀 **Recent Activities:** Tracks recently opened applications.
    * 🧪 **CPU Benchmark:** Test your processor's raw power.
    * 🌐 **Latency Test:** Quick ping to check your internet stability.
* **Advanced Specs:** Smart GPU detection, Drive storage visualization, and organized Peripheral list (Monitors, Audio, Inputs...).
* **Customization:** Fully themeable interface.

### 🛠️ Installation

1.  **Clone the repo**
    ```bash
    git clone [https://github.com/BateMane/geewerscontrolroom.git](https://github.com/BateMane/geewerscontrolroom.git)
    cd geewer-hw-monitor
    ```

2.  **Install dependencies**
    ```bash
    npm install
    ```

3.  **Run it**
    ```bash
    npm run tauri dev
    ```