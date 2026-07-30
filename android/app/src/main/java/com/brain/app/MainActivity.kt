package com.brain.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.brain.app.ui.AgentChatScreen
import com.brain.app.ui.ModelEditorScreen
import com.brain.app.ui.ProviderDetailScreen
import com.brain.app.ui.ProvidersScreen
import com.brain.app.ui.SettingsScreen
import com.brain.app.ui.theme.BrainTheme
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.*

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val settings = BrainSettings(this)
        val repository = AgentRepository(settings)

        setContent {
            BrainTheme {
                val vm: AgentViewModel = viewModel(
                    factory = AgentViewModelFactory(application, repository, settings)
                )
                var showSettings by remember { mutableStateOf(!settings.isConfigured) }
                var showProviders by remember { mutableStateOf(false) }
                var providerDetailId by remember { mutableStateOf<Long?>(null) }
                var providerDetailName by remember { mutableStateOf("") }
                var modelEditProviderId by remember { mutableStateOf<Long?>(null) }
                var modelEditId by remember { mutableStateOf("") }

                when {
                    modelEditProviderId != null -> ModelEditorScreen(
                        settings = settings,
                        providerId = modelEditProviderId!!,
                        modelId = modelEditId,
                        onBack = { modelEditProviderId = null; modelEditId = "" },
                    )
                    providerDetailId != null -> ProviderDetailScreen(
                        settings = settings,
                        providerId = providerDetailId!!,
                        providerName = providerDetailName,
                        onBack = { providerDetailId = null; providerDetailName = "" },
                        onModelEdit = { pid, mid -> modelEditProviderId = pid; modelEditId = mid },
                    )
                    showProviders -> ProvidersScreen(
                        settings = settings,
                        onBack = { showProviders = false },
                        onProviderClick = { id, name -> providerDetailId = id; providerDetailName = name },
                    )
                    showSettings -> SettingsScreen(
                        settings = settings,
                        onBack = { showSettings = false },
                        onProviders = { showProviders = true },
                    )
                    else -> {
                        val drawerState = rememberDrawerState(initialValue = DrawerValue.Closed)
                        val scope = rememberCoroutineScope()

                        ModalNavigationDrawer(
                            drawerState = drawerState,
                            drawerContent = {
                                ModalDrawerSheet(
                                    modifier = Modifier.width(300.dp),
                                    drawerContainerColor = Color(0xFF000000)
                                ) {
                                    DrawerContent(
                                        vm = vm,
                                        onNewChat = { vm.newChat() },
                                        onSelectChat = { vm.selectChat(it); scope.launch { drawerState.close() } },
                                        onDeleteChat = { vm.deleteChat(it) },
                                        onSettings = { showSettings = true }
                                    )
                                }
                            }
                        ) {
                            val events by vm.events.collectAsState()
                            val isRunning by vm.isRunning.collectAsState()
                            val selectedModel by vm.selectedModel.collectAsState()
                            val availableModels by vm.availableModels.collectAsState()
                            val chats by vm.chats.collectAsState()
                            val currentId by vm.currentChatId.collectAsState()
                            val currentChat = chats.find { it.id == currentId }

                            AgentChatScreen(
                                events = events,
                                isRunning = isRunning,
                                selectedModel = selectedModel,
                                chatTitle = currentChat?.title ?: "Новый чат",
                                availableModels = availableModels,
                                onSendTask = { task -> vm.sendTask(task) },
                                onStop = { vm.stopAgent() },
                                onMenuClick = { scope.launch { drawerState.open() } },
                                onNewChat = { vm.newChat() },
                                onSettings = { showSettings = true },
                                onModelSelected = { vm.selectModel(it) }
                            )
                        }
                    }
                }
            }
        }
    }
}

class AgentViewModelFactory(
    private val app: android.app.Application,
    private val repository: AgentRepository,
    private val settings: BrainSettings
) : androidx.lifecycle.ViewModelProvider.Factory {
    @Suppress("UNCHECKED_CAST")
    override fun <T : androidx.lifecycle.ViewModel> create(modelClass: Class<T>): T {
        return AgentViewModel(app, repository, settings) as T
    }
}

@Composable
fun DrawerContent(
    vm: AgentViewModel,
    onNewChat: () -> Unit,
    onSelectChat: (String) -> Unit,
    onDeleteChat: (String) -> Unit,
    onSettings: () -> Unit
) {
    val chats by vm.chats.collectAsState()
    val currentId by vm.currentChatId.collectAsState()

    Column(modifier = Modifier.fillMaxSize().background(Color(0xFF000000))) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text("Brain", fontWeight = FontWeight.Bold, fontSize = 20.sp, color = Color.White)
            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                IconButton(onClick = onNewChat, modifier = Modifier.size(36.dp)) {
                    Icon(Icons.Default.Add, "Новый чат", modifier = Modifier.size(20.dp))
                }
                IconButton(onClick = onSettings, modifier = Modifier.size(36.dp)) {
                    Icon(Icons.Default.Settings, "Настройки", modifier = Modifier.size(20.dp))
                }
            }
        }

        HorizontalDivider(color = Color(0xFF222222))

        val grouped = chats.groupBy { chat ->
            val cal = Calendar.getInstance().apply { timeInMillis = chat.updatedAt }
            val now = Calendar.getInstance()
            when {
                cal.get(Calendar.YEAR) == now.get(Calendar.YEAR) &&
                        cal.get(Calendar.DAY_OF_YEAR) == now.get(Calendar.DAY_OF_YEAR) -> "Сегодня"
                cal.get(Calendar.YEAR) == now.get(Calendar.YEAR) &&
                        cal.get(Calendar.DAY_OF_YEAR) == now.get(Calendar.DAY_OF_YEAR) - 1 -> "Вчера"
                else -> SimpleDateFormat("d MMM", Locale("ru")).format(Date(chat.updatedAt))
            }
        }

        LazyColumn(modifier = Modifier.weight(1f)) {
            grouped.forEach { (dateLabel, dayChats) ->
                item {
                    Text(
                        dateLabel, style = MaterialTheme.typography.labelMedium,
                        color = Color(0xFF666666),
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
                    )
                }
                items(dayChats, key = { it.id }) { chat ->
                    val isSelected = chat.id == currentId
                    Surface(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 8.dp, vertical = 2.dp)
                            .clip(RoundedCornerShape(10.dp))
                            .clickable { onSelectChat(chat.id) },
                        color = if (isSelected) MaterialTheme.colorScheme.primary.copy(alpha = 0.15f)
                        else Color.Transparent
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                chat.title,
                                color = if (isSelected) MaterialTheme.colorScheme.primary else Color.White,
                                fontSize = 14.sp, maxLines = 1, overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.weight(1f)
                            )
                        }
                    }
                }
            }
        }
    }
}
